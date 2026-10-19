use alexandria::cli::{
    CliCommand, HELP_BANNER, VERSION_BANNER, execute_autonomous_run, execute_init_scaffolding,
    parse_cli_args, resolve_default_vault_db_path,
};
use alexandria::mcp::server::run_mcp_stdio_loop;
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use alexandria::storage::actor::VaultStorageHandle;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cmd = match parse_cli_args(env::args()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ALEXANDRIA KERNEL ERROR] {}", e);
            return ExitCode::from(2);
        }
    };

    match cmd {
        CliCommand::Version => {
            println!("{}", VERSION_BANNER);
            return ExitCode::SUCCESS;
        }
        CliCommand::Help => {
            println!("{}", HELP_BANNER);
            return ExitCode::SUCCESS;
        }
        _ => {}
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("alexandria-tokio-worker")
        .enable_all()
        .build()
        .expect("Fallo crítico inicializando el reactor Tokio");

    let res: Result<(), String> = runtime.block_on(async move {
        match cmd {
            CliCommand::Version | CliCommand::Help => Ok(()),
            CliCommand::Init {
                vault_path,
                profile,
            } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Error obteniendo directorio actual: {}", e))?;
                let config_path = execute_init_scaffolding(
                    &cwd,
                    vault_path.as_deref(),
                    profile.as_deref(),
                )?;
                println!(
                    "[ALEXANDRIA KERNEL] Espacio de trabajo inicializado exitosamente: {}",
                    config_path.display()
                );
                Ok(())
            }
            CliCommand::McpStdio => {
                let db_path = resolve_default_vault_db_path()?;
                let _storage_handle = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)?;
                run_mcp_stdio_loop().await
            }
            CliCommand::Run { instruction } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Error obteniendo directorio actual: {}", e))?;
                let report = execute_autonomous_run(&cwd, &instruction)?;
                println!("{}", report);
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
