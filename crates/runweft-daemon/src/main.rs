use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [argument] if argument == "--version" => {
            println!("runweftd {} (scaffold)", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [argument] if argument == "--help" => {
            println!(
                "Runweft coordinator placeholder. No daemon or socket is started. See spec.md."
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("Coordinator not implemented; refusing to start. No execution is available.");
            ExitCode::from(2)
        }
    }
}
