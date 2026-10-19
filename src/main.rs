use alexandria::mcp::server::run_mcp_stdio_loop;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
pub enum CliCommand {
    Init { vault_path: PathBuf },
    McpStdio,
    Run { instruction: String },
}

pub fn parse_cli_args<I>(args: I) -> Result<CliCommand, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let _bin = iter.next();

    match iter.next().as_deref() {
        Some("init") => {
            let vault_path = iter
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./alexandria"));
            Ok(CliCommand::Init { vault_path })
        }
        Some("mcp") | Some("serve") => Ok(CliCommand::McpStdio),
        Some("run") => {
            let parts: Vec<String> = iter.collect();
            if parts.is_empty() {
                return Err("El comando 'run' requiere una directiva de ejecución.".into());
            }
            Ok(CliCommand::Run {
                instruction: parts.join(" "),
            })
        }
        Some(other) => Err(format!(
            "Subcomando desconocido '{}'. Uso: alexandria <init|mcp|run>",
            other
        )),
        None => Err("Uso: alexandria <init|mcp|run>".into()),
    }
}

fn main() -> ExitCode {
    let cmd = match parse_cli_args(env::args()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ALEXANDRIA KERNEL ERROR] {}", e);
            return ExitCode::from(2);
        }
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("alexandria-tokio-worker")
        .enable_all()
        .build()
        .expect("Fallo crítico inicializando el reactor Tokio");

    let res: Result<(), String> = runtime.block_on(async move {
        match cmd {
            CliCommand::Init { vault_path } => {
                std::fs::create_dir_all(vault_path.join("architecture"))
                    .and_then(|_| std::fs::create_dir_all(vault_path.join("modules")))
                    .and_then(|_| std::fs::create_dir_all(vault_path.join("decisions")))
                    .and_then(|_| std::fs::create_dir_all(vault_path.join("raw")))
                    .map_err(|e| format!("Error creando estructura de bóveda: {}", e))?;
                eprintln!("[ALEXANDRIA KERNEL] Bóveda inicializada en {:?}", vault_path);
                Ok(())
            }
            CliCommand::McpStdio => run_mcp_stdio_loop().await,
            CliCommand::Run { instruction } => {
                eprintln!(
                    "[ALEXANDRIA KERNEL] Ejecutando directiva en modo autónomo: {}",
                    instruction
                );
                Ok(())
            }
        }
    });

    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[ALEXANDRIA KERNEL ABORT] {}", err);
            ExitCode::from(1)
        }
    }
}
