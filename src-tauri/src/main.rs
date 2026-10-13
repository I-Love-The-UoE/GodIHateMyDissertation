#![windows_subsystem = "windows"]

fn main() -> Result<(), gihmd_lib::StartupError> {
    gihmd_lib::run()
}
