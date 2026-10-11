//! `axetwo`: inspect, set up and test an Axe-Fx II on USB.

#[cfg(windows)]
mod cli;
#[cfg(windows)]
mod wav;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("axetwo currently runs on Windows only.");
}
