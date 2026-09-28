use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["--help"] | ["-h"] => {
            println!(
                "Runweft scaffold\n\nUsage: runweft status [--json] | --version\n\nExecution is not implemented. See spec.md."
            );
            ExitCode::SUCCESS
        }
        ["--version"] | ["-V"] => {
            println!("runweft {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["status", "--json"] => match serde_json::to_string_pretty(&runweft_core::status()) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Cannot encode scaffold status: {error}");
                ExitCode::FAILURE
            }
        },
        ["status"] => {
            println!("Phase: scaffold\nExecution: disabled\nCoordinator: not implemented");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "Unsupported command. This scaffold cannot execute or resume runs. Use --help."
            );
            ExitCode::from(2)
        }
    }
}
