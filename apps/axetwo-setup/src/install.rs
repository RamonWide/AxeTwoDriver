//! The install and uninstall steps.

use crate::packages;
use axetwo_stream::status::{self, UnitState};
use axetwo_stream::{AUDIO_FUNCTION_ID, INTERFACE_GUID};
use axetwo_win::registry::{self, Root};
use axetwo_win::{elevate, setup, winusb};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

/// Programs installed when they sit next to the installer.
const PROGRAMS: [&str; 3] = ["axetwo.exe", "axetwo-panel.exe", "axetwo-setup.exe"];
/// Optional audio-driver libraries, registered with Windows when present.
const LIBRARIES: [&str; 1] = ["axetwo_daw.dll"];
const DOCUMENTS: [&str; 6] =
    ["LICENSE.txt", "README.txt", "LICENSE-DAW.txt", "ATTRIBUTION.txt", "SOURCE.txt", "THIRD-PARTY-LICENSES.txt"];
/// The original audio driver's packages. Its firmware loader is not listed
/// and is never touched.
const ORIGINAL_PACKAGES: [&str; 2] = ["fasusbaudio.inf", "fasusbaudioks.inf"];

const UNINSTALL_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\AxeTwoDriver";
const SHORTCUT: &str = "AxeTwoDriver Control Panel.lnk";
const DEVICE_WAIT: Duration = Duration::from_secs(20);
/// Process creation flag: run without attaching to the parent's console.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct Log {
    file: Option<std::fs::File>,
    problems: u32,
}

impl Log {
    fn say(&mut self, text: &str) {
        println!("{text}");
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "{text}");
        }
    }

    fn problem(&mut self, text: &str) {
        self.problems += 1;
        self.say(&format!("  ! {text}"));
    }
}

fn env_dir(name: &str, fallback: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| fallback.to_string()))
}

fn install_dir() -> PathBuf {
    env_dir("ProgramFiles", "C:\\Program Files").join("AxeTwoDriver")
}

fn data_dir() -> PathBuf {
    env_dir("ProgramData", "C:\\ProgramData").join("AxeTwoDriver")
}

fn shortcut_path() -> PathBuf {
    data_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
        .join("Microsoft\\Windows\\Start Menu\\Programs")
        .join(SHORTCUT)
}

/// Runs a Windows tool and returns whether it succeeded and what it printed.
fn tool(program: &str, args: &[&str]) -> (bool, String) {
    match Command::new(program).args(args).output() {
        Ok(out) => (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned()),
        Err(e) => (false, e.to_string()),
    }
}

pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let quiet = args.iter().any(|a| a == "--quiet");
    let uninstall = args.iter().any(|a| a == "uninstall");

    if !elevate::is_elevated() {
        let Ok(exe) = std::env::current_exe() else { return ExitCode::FAILURE };
        return match elevate::run_elevated(&exe.to_string_lossy(), &args.join(" ")) {
            Ok(0) => ExitCode::SUCCESS,
            Ok(_) => ExitCode::FAILURE,
            Err(e) => {
                eprintln!("AxeTwoDriver needs administrator rights to install: {e}");
                pause(quiet);
                ExitCode::FAILURE
            }
        };
    }

    let _ = std::fs::create_dir_all(data_dir());
    let file = std::fs::OpenOptions::new().create(true).append(true).open(data_dir().join("install.log")).ok();
    let mut log = Log { file, problems: 0 };
    log.say(&format!("AxeTwoDriver {} {}", env!("CARGO_PKG_VERSION"), if uninstall { "uninstall" } else { "install" }));
    if uninstall {
        run_uninstall(&mut log);
    } else {
        run_install(&mut log);
    }
    let ok = log.problems == 0;
    log.say(if ok { "\nDone." } else { "\nFinished with problems; see the lines marked with ! above." });
    pause(quiet);
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn pause(quiet: bool) {
    if !quiet {
        println!("\nPress Enter to close this window.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

fn run_install(log: &mut Log) {
    let dest = install_dir();
    log.say("\n1. Copying the programs");
    copy_files(log, &dest);

    log.say("\n2. Taking the original audio driver off the unit");
    remove_original_driver(log);

    log.say("\n3. Setting up the unit's audio");
    set_up_audio(log);

    log.say("\n4. Registering with Windows");
    for library in LIBRARIES {
        let path = dest.join(library);
        if path.exists() {
            let (ok, _) = tool("regsvr32", &["/s", &path.to_string_lossy()]);
            if ok {
                log.say(&format!("  audio driver for DAWs registered ({library})"));
            } else {
                log.problem(&format!("{library} could not be registered"));
            }
        }
    }
    register_uninstaller(log, &dest);
    create_shortcut(log, &dest);
}

fn copy_files(log: &mut Log, dest: &Path) {
    let Some(source) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) else {
        log.problem("the installer could not find its own folder");
        return;
    };
    if source == dest {
        log.say("  already running from the install folder");
        return;
    }
    if let Err(e) = std::fs::create_dir_all(dest) {
        log.problem(&format!("could not create {}: {e}", dest.display()));
        return;
    }
    for name in PROGRAMS.iter().chain(&LIBRARIES).chain(&DOCUMENTS) {
        let from = source.join(name);
        if !from.exists() {
            continue;
        }
        match std::fs::copy(&from, dest.join(name)) {
            Ok(_) => log.say(&format!("  {name}")),
            Err(e) => log.problem(&format!(
                "{name} could not be copied ({e}). Close your audio programs and run the installer again."
            )),
        }
    }
    log.say(&format!("  into {}", dest.display()));
}

fn remove_original_driver(log: &mut Log) {
    let (ok, listing) = tool("pnputil", &["/enum-drivers"]);
    if !ok {
        log.problem("Windows' driver list could not be read");
        return;
    }
    let found = packages::find(&listing, &ORIGINAL_PACKAGES);
    if found.is_empty() {
        log.say("  not installed; nothing to do");
        return;
    }
    let backup = data_dir().join("original-driver");
    // Back everything up before removing anything.
    for (oem, name) in &found {
        let dir = backup.join(name.trim_end_matches(".inf"));
        let _ = std::fs::create_dir_all(&dir);
        let (ok, _) = tool("pnputil", &["/export-driver", oem, &dir.to_string_lossy()]);
        if !ok || !dir.join(name).exists() {
            log.problem(&format!("{name} could not be backed up, so nothing was removed"));
            return;
        }
    }
    log.say(&format!("  backed up to {}", backup.display()));
    for (oem, name) in &found {
        let (ok, _) = tool("pnputil", &["/delete-driver", oem, "/uninstall"]);
        if ok {
            log.say(&format!("  removed {name}"));
        } else {
            log.problem(&format!("{name} could not be removed"));
        }
    }
    tool("pnputil", &["/scan-devices"]);
}

fn set_up_audio(log: &mut Log) {
    if winusb::interface_paths(INTERFACE_GUID).is_ok_and(|p| !p.is_empty()) {
        log.say("  already set up");
        return;
    }
    // After the original driver is removed Windows needs a moment to show the
    // unit's audio function again.
    let started = Instant::now();
    let present = loop {
        if setup::find_instance(AUDIO_FUNCTION_ID).is_ok_and(|i| i.is_some()) {
            break true;
        }
        if started.elapsed() > DEVICE_WAIT {
            break false;
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    if !present {
        match status::read().map(|s| s.unit) {
            Ok(UnitState::NeedsFirmware) => log.problem(
                "the unit is connected but its USB firmware was not loaded. Install Fractal Audio's driver package (its firmware loader is still needed), then run this installer again.",
            ),
            Ok(UnitState::Running) => {
                log.problem("Windows does not show the unit's audio function. Unplug the USB cable, plug it back in and run this installer again.")
            }
            _ => log.problem("no Axe-Fx II found. Connect it by USB, switch it on and run this installer again."),
        }
        return;
    }
    match setup::bind_winusb(AUDIO_FUNCTION_ID, INTERFACE_GUID) {
        Ok(report) => {
            log.say(&format!("  {} now uses Windows' own {}", report.instance_id, report.driver));
            if report.reboot_required {
                log.say("  Windows asks for a restart to finish.");
            }
        }
        Err(e) => log.problem(&e.to_string()),
    }
}

fn register_uninstaller(log: &mut Log, dest: &Path) {
    let uninstall = format!("\"{}\" uninstall", dest.join("axetwo-setup.exe").display());
    let strings = [
        ("DisplayName", "AxeTwoDriver"),
        ("DisplayVersion", env!("CARGO_PKG_VERSION")),
        ("Publisher", "The AxeTwoDriver contributors"),
        ("InstallLocation", &dest.to_string_lossy()),
        ("UninstallString", &uninstall),
    ];
    let result = strings
        .iter()
        .try_for_each(|(name, value)| registry::set_string(Root::LocalMachine, UNINSTALL_KEY, name, value))
        .and_then(|()| registry::set_dword(Root::LocalMachine, UNINSTALL_KEY, "NoModify", 1))
        .and_then(|()| registry::set_dword(Root::LocalMachine, UNINSTALL_KEY, "NoRepair", 1));
    match result {
        Ok(()) => log.say("  listed under Installed apps"),
        Err(e) => log.problem(&format!("could not add the Installed apps entry: {e}")),
    }
}

/// A Start menu shortcut to the control panel. Windows' script host writes
/// the shortcut file.
fn create_shortcut(log: &mut Log, dest: &Path) {
    let panel = dest.join("axetwo-panel.exe");
    if !panel.exists() {
        return;
    }
    let quote = |p: &Path| p.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.Save()",
        quote(&shortcut_path()),
        quote(&panel)
    );
    let (ok, _) = tool("powershell", &["-NoProfile", "-NonInteractive", "-Command", &script]);
    if ok {
        log.say("  control panel added to the Start menu");
    } else {
        log.say("  (no Start menu shortcut; open axetwo-panel.exe from the install folder)");
    }
}

fn run_uninstall(log: &mut Log) {
    let dest = install_dir();
    log.say("\n1. Unregistering");
    for library in LIBRARIES {
        let path = dest.join(library);
        if path.exists() {
            tool("regsvr32", &["/u", "/s", &path.to_string_lossy()]);
            log.say(&format!("  {library}"));
        }
    }
    let _ = std::fs::remove_file(shortcut_path());
    if let Err(e) = registry::delete_tree(Root::LocalMachine, UNINSTALL_KEY) {
        log.problem(&e.to_string());
    }

    log.say("\n2. Returning the unit's audio to Windows");
    match setup::find_instance(AUDIO_FUNCTION_ID) {
        Ok(Some(instance)) => {
            let (ok, _) = tool("pnputil", &["/remove-device", &instance]);
            tool("pnputil", &["/scan-devices"]);
            if ok {
                log.say("  done");
            } else {
                log.problem("the device could not be reset; use Uninstall device in Device Manager");
            }
        }
        _ => log.say("  the unit is not connected; if it is still set up, use Uninstall device in Device Manager"),
    }

    log.say("\n3. Removing the programs");
    // A running program cannot delete itself, so a short-lived command prompt
    // removes the folder once this one has closed.
    let command = format!("ping -n 3 127.0.0.1 >nul & rmdir /s /q \"{}\"", dest.display());
    // Its own hidden console, so it outlives this window.
    match Command::new("cmd").args(["/c", &command]).current_dir(data_dir()).creation_flags(CREATE_NO_WINDOW).spawn() {
        Ok(_) => log.say(&format!("  {}", dest.display())),
        Err(e) => log.problem(&format!("could not remove {}: {e}", dest.display())),
    }
    let backup = data_dir().join("original-driver");
    if backup.exists() {
        log.say(&format!(
            "\nThe original audio driver was kept in {}.\nTo put it back: pnputil /add-driver \"{}\\fasusbaudio\\fasusbaudio.inf\" /install",
            backup.display(),
            backup.display()
        ));
    }
}
