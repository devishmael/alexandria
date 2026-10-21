use alexandria::cli::{
    execute_autonomous_run, execute_capsule_command, execute_collect_subcommand,
    execute_doctor_diagnostic, execute_gate_check, execute_harness_scaffold,
    execute_init_scaffolding, execute_memory_subcommand, execute_vault_command, parse_cli_args,
    resolve_default_vault_db_path, CliCommand, HELP_BANNER, VERSION_BANNER,
};
use alexandria::lsp::server::run_lsp_stdio_loop;
use alexandria::mcp::server::{run_mcp_stdio_loop_with_context, McpKernelContext};
use alexandria::storage::actor::VaultStorageHandle;
use alexandria::storage::CANONICAL_SCHEMA_SQL;
use std::env;
use std::path::{Path, PathBuf};
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
        .expect("Critical failure initializing Tokio runtime");

    let res: Result<(), String> = runtime.block_on(async move {
        match cmd {
            CliCommand::Version | CliCommand::Help => Ok(()),
            CliCommand::Init {
                vault_path,
                profile,
            } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let config_path = execute_init_scaffolding(
                    &cwd,
                    vault_path.as_deref(),
                    profile.as_deref(),
                )?;
                println!(
                    "[ALEXANDRIA KERNEL] Workspace initialized successfully: {}",
                    config_path.display()
                );
                Ok(())
            }
            CliCommand::Doctor => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let report = execute_doctor_diagnostic(&cwd)?;
                println!("{}", report);
                Ok(())
            }
            CliCommand::Gate { target_path } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let path = if Path::new(&target_path).is_absolute() {
                    PathBuf::from(&target_path)
                } else {
                    cwd.join(&target_path)
                };
                let (passed, report) = execute_gate_check(&path)?;
                println!("{}", report);
                if !passed {
                    return Err("Quality gate violation detected".to_string());
                }
                Ok(())
            }
            CliCommand::Capsule {
                subaction,
                argument,
                domain,
            } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let report = execute_capsule_command(
                    &cwd,
                    &subaction,
                    argument.as_deref(),
                    domain.as_deref(),
                )?;
                println!("{}", report);
                Ok(())
            }
            CliCommand::Harness { target } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let report = execute_harness_scaffold(&cwd, &target)?;
                println!("{}", report);
                Ok(())
            }
            CliCommand::Vault { action } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let report = execute_vault_command(&cwd, &action)?;
                println!("{}", report);
                Ok(())
            }
            CliCommand::Collect {
                source_dir,
                output_dir,
            } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let db_path = resolve_default_vault_db_path()?;
                let storage_handle = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)?;
                let report = execute_collect_subcommand(
                    &cwd,
                    &source_dir,
                    output_dir.as_deref(),
                    Some(&storage_handle),
                )
                .await?;
                println!("{}", report);
                Ok(())
            }
            CliCommand::McpStdio => {
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let vault_dir = if cwd.join("alexandria").exists() {
                    cwd.join("alexandria")
                } else {
                    cwd.join("wiki")
                };
                let db_path = resolve_default_vault_db_path()?;
                let storage_handle = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)?;
                let ctx = McpKernelContext::new(cwd, vault_dir, Some(storage_handle));
                run_mcp_stdio_loop_with_context(ctx).await
            }
            CliCommand::LspStdio => run_lsp_stdio_loop().await,
            CliCommand::Memory {
                action,
                text,
                source_kind,
                module_name,
                token_budget,
            } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
                let db_path = resolve_default_vault_db_path()?;
                let storage_handle = VaultStorageHandle::spawn(db_path, CANONICAL_SCHEMA_SQL)?;
                let report = execute_memory_subcommand(
                    &cwd,
                    &action,
                    &text,
                    source_kind.as_deref(),
                    module_name.as_deref(),
                    token_budget,
                    &storage_handle,
                )
                .await?;
                println!("{}", report);
                Ok(())
            }
            CliCommand::Run { instruction } => {
                let cwd = env::current_dir()
                    .map_err(|e| format!("Failed to resolve current working directory: {}", e))?;
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
