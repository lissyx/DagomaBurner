//! `cargo run -p dagomapp-frontend -- build|dev`, used by `cargo tauri dev` and
//! `cargo tauri build` (see src-tauri/tauri.conf.json).

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [command] = args.as_slice() else {
        eprintln!("usage: dagomapp-frontend build|dev");
        return ExitCode::from(2);
    };

    let mut cmd = match dagomapp_frontend::command(&root, command) {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Become the Vite process, so stopping `cargo tauri dev` stops the dev server.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = cmd.exec();
        eprintln!("error: can't run Node.js: {error}");
        ExitCode::FAILURE
    }
    #[cfg(not(unix))]
    match cmd.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("error: can't run Node.js: {error}");
            ExitCode::FAILURE
        }
    }
}
