use std::env;
use std::process;

const USAGE: &str = "usage: eudkit-lsp [--stdio] [--version] [--help]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliAction {
    RunServer,
    PrintVersion,
    PrintHelp,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match parse_args(env::args().skip(1)) {
        Ok(CliAction::RunServer) => match eudkit_lsp::server::run() {
            Ok(()) => Ok(()),
            // A client that closes stdin before or during initialize is a
            // normal shutdown, not a protocol failure.
            Err(error) if is_client_disconnect(error.as_ref()) => Ok(()),
            Err(error) => Err(error),
        },
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

fn is_client_disconnect(error: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    error
        .downcast_ref::<lsp_server::ProtocolError>()
        .is_some_and(lsp_server::ProtocolError::channel_is_disconnected)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<CliAction, String> {
    let args: Vec<String> = args.into_iter().collect();
    match args.as_slice() {
        [] => Ok(CliAction::RunServer),
        [arg] => match arg.as_str() {
            // LSP clients such as vscode-languageclient pass `--stdio` for
            // TransportKind.stdio. The server always speaks stdio, so accept
            // the flag as a no-op instead of failing the client's spawn.
            "--stdio" => Ok(CliAction::RunServer),
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
    fn stdio_flag_starts_the_stdio_server() {
        assert_eq!(
            parse_args(["--stdio".to_string()]).unwrap(),
            CliAction::RunServer
        );
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
