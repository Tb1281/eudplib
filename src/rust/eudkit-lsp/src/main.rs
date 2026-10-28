use std::env;
use std::process;

const USAGE: &str = "usage: eudkit-lsp [--version] [--help]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliAction {
    RunServer,
    PrintVersion,
    PrintHelp,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match parse_args(env::args().skip(1)) {
        Ok(CliAction::RunServer) => eudkit_lsp::server::run(),
        Ok(CliAction::PrintVersion) => {
            println!("eudkit-lsp {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Ok(CliAction::PrintHelp) => {
            println!("{USAGE}");
            Ok(())
        }
        Err(message) => {
            eprintln!("error: {message}\n{USAGE}");
            process::exit(2);
        }
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliAction, String> {
    let args: Vec<String> = args.into_iter().collect();
    match args.as_slice() {
        [] => Ok(CliAction::RunServer),
        [arg] => match arg.as_str() {
            "--version" | "-V" => Ok(CliAction::PrintVersion),
            "--help" | "-h" => Ok(CliAction::PrintHelp),
            other => Err(format!("unknown argument '{other}'")),
        },
        [first, ..] => Err(format!("unexpected arguments after '{first}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_starts_the_stdio_server() {
        assert_eq!(parse_args([]).unwrap(), CliAction::RunServer);
    }

    #[test]
    fn recognizes_version_and_help_flags() {
        assert_eq!(
            parse_args(["--version".to_string()]).unwrap(),
            CliAction::PrintVersion
        );
        assert_eq!(
            parse_args(["-V".to_string()]).unwrap(),
            CliAction::PrintVersion
        );
        assert_eq!(
            parse_args(["--help".to_string()]).unwrap(),
            CliAction::PrintHelp
        );
        assert_eq!(
            parse_args(["-h".to_string()]).unwrap(),
            CliAction::PrintHelp
        );
    }

    #[test]
    fn rejects_unknown_or_extra_arguments() {
        let unknown = parse_args(["--listen".to_string()]).unwrap_err();
        assert!(unknown.contains("--listen"), "{unknown}");

        let extra = parse_args(["--version".to_string(), "extra".to_string()]).unwrap_err();
        assert!(extra.contains("unexpected arguments"), "{extra}");
    }
}
