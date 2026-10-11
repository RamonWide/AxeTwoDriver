//! The AxeTwoDriver installer. Double-click it: it asks for administrator
//! rights, sets the unit up and installs the programs that sit next to it.
//!
//!   axetwo-setup [install | uninstall] [--quiet]

mod packages;

#[cfg(windows)]
mod install;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    install::main()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("AxeTwoDriver installs on Windows only.");
}
