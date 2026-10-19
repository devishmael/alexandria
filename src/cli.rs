use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub const VERSION_BANNER: &str = "alexandria-runtime v2.4.0-OmniPragmatic";

pub const HELP_BANNER: &str = "\
alexandria-runtime v2.4.0-OmniPragmatic
Headless Deterministic Agent Harness & Kernel-Space Arbitrator

USO:
  alexandria <COMANDO> [OPCIONES]

OPCIONES GLOBALES:
  -v, --version                  Imprime la versión exacta del kernel y termina.
  -h, --help                     Muestra este catálogo de ayuda y termina.

COMANDOS:
  init [--vault <ruta>] [--profile <perfil>]
                                 Inicializa .alexandria.json, la bóveda documental
                                 (wiki/index.md, log.md, failures.md, architecture/,
                                 modules/, decisions/) y blinda el archivo .gitignore.
  mcp                            Inicia el actor SQLite WAL en ~/.alexandria/vault.db
                                 y expone el servidor MCP JSON-RPC 2.0 sobre stdio.
  run \"<directiva>\"              Ejecuta una directiva en modo CLI autónomo evaluando
                                 la ruta de ejecución (Path Alpha / Path Omega).";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Version,
    Help,
    Init {
        vault_path: Option<String>,
        profile: Option<String>,
    },
    McpStdio,
    Run {
        instruction: String,
    },
}

pub fn parse_cli_args<I>(args: I) -> Result<CliCommand, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let _bin = iter.next();

    match iter.next().as_deref() {
        Some("--version") | Some("-v") => Ok(CliCommand::Version),
        Some("--help") | Some("-h") => Ok(CliCommand::Help),
        Some("init") => {
            let mut vault_path: Option<String> = None;
            let mut profile: Option<String> = None;

            while let Some(arg) = iter.next() {
                if arg == "--vault" {
                    let val = iter.next().ok_or_else(|| {
                        "La bandera '--vault' requiere especificar una ruta.".to_string()
                    })?;
                    vault_path = Some(val);
                } else if let Some(val) = arg.strip_prefix("--vault=") {
                    vault_path = Some(val.to_string());
                } else if arg == "--profile" {
                    let val = iter.next().ok_or_else(|| {
                        "La bandera '--profile' requiere especificar un perfil.".to_string()
                    })?;
                    profile = Some(val);
                } else if let Some(val) = arg.strip_prefix("--profile=") {
                    profile = Some(val.to_string());
                } else if arg == "--help" || arg == "-h" {
                    return Ok(CliCommand::Help);
                } else if !arg.starts_with('-') && vault_path.is_none() {
                    vault_path = Some(arg);
                } else {
                    return Err(format!("Opción desconocida en 'init': '{}'", arg));
                }
            }

            Ok(CliCommand::Init {
                vault_path,
                profile,
            })
        }
        Some("mcp") | Some("serve") => Ok(CliCommand::McpStdio),
        Some("run") => {
            let parts: Vec<String> = iter.collect();
            if parts.is_empty() || parts.iter().all(|s| s.trim().is_empty()) {
                return Err("El comando 'run' requiere una directiva de ejecución.".into());
            }
            Ok(CliCommand::Run {
                instruction: parts.join(" "),
            })
        }
        Some(other) => Err(format!(
            "Subcomando desconocido '{}'. Uso: alexandria <init|mcp|run|--version|--help>",
            other
        )),
        None => Err("Uso: alexandria <init|mcp|run|--version|--help>".into()),
    }
}

pub fn execute_init_scaffolding(
    target_root: &Path,
    custom_vault: Option<&str>,
    profile: Option<&str>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(target_root)
        .map_err(|e| format!("No se pudo crear el directorio raíz {:?}: {}", target_root, e))?;

    let configured_vault_str = custom_vault.unwrap_or("./wiki");
    let configured_profile = profile.unwrap_or("mvp-fullstack-ts");

    // Siempre creamos el árbol base local `wiki/` en la raíz objetivo
    let wiki_dir = target_root.join("wiki");
    for subdir in ["architecture", "modules", "decisions", "raw"] {
        fs::create_dir_all(wiki_dir.join(subdir))
            .map_err(|e| format!("Error creando subdirectorio wiki/{}: {}", subdir, e))?;
    }

    let index_path = wiki_dir.join("index.md");
    if !index_path.exists() {
        let index_content = "---\nid: index\ntitle: \"Alexandria Vault — Índice Maestro\"\ntype: index\nstatus: canonical\nversion: \"2.4.0-OmniPragmatic\"\n---\n\n# Alexandria Vault — Índice Maestro\n\n- Bitácora Canónica: [[log]]\n- Registro Forense: [[failures]]\n";
        fs::write(&index_path, index_content)
            .map_err(|e| format!("Error escribiendo wiki/index.md: {}", e))?;
    }

    let log_path = wiki_dir.join("log.md");
    if !log_path.exists() {
        let log_content = "---\nid: log\ntitle: \"Bitácora de Cambios Aprobados\"\ntype: operational-log\nstatus: canonical\n---\n\n# Bitácora de Cambios Aprobados\n\n- Mapa Maestro: [[index]]\n";
        fs::write(&log_path, log_content)
            .map_err(|e| format!("Error escribiendo wiki/log.md: {}", e))?;
    }

    let failures_path = wiki_dir.join("failures.md");
    if !failures_path.exists() {
        let failures_content = "---\nid: failures\ntitle: \"Registro Forense de Incidentes y Circuit Breaker\"\ntype: forensic-registry\nstatus: canonical\n---\n\n# Registro Forense de Incidentes (`FAIL-XXX`)\n\n- Mapa Maestro: [[index]]\n";
        fs::write(&failures_path, failures_content)
            .map_err(|e| format!("Error escribiendo wiki/failures.md: {}", e))?;
    }

    let project_namespace = target_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("alexandria-project");

    let config_json = json!({
        "$schema": "https://alexandria.dev/schema.v2.4.json",
        "projectNamespace": project_namespace,
        "profile": configured_profile,
        "vaultPath": configured_vault_str,
        "fileWatcherDebounceMs": 75,
        "engine": {
            "strictMode": true,
            "moduleContextDepth": 1,
            "maxTransitiveAdrHoistingDepth": 6,
            "circuitBreakerMaxRetries": 3,
            "protectedPathsForOmegaUpgrade": [
                "prisma/schema.prisma",
                "src/core/auth/**",
                "alexandria/decisions/**"
            ],
            "oxcAstInvariants": {
                "forbiddenTypes": ["any"],
                "forbidDoubleCastUnknown": true,
                "enforceInlinePrismaTransactions": true,
                "enforceTxOnlyAwaitsInPrismaTransaction": true,
                "forbidTxAliasingOrDestructuring": true,
                "forbidUnawaitedCallsInPrismaTransaction": true,
                "promoteToOmegaOnPublicExportSignatureChange": true
            },
            "sandbox": {
                "clearEnvVars": true,
                "preserveWindowsSystemRoot": true,
                "stripWindowsUncPrefix": true,
                "directNodeVitestInvocation": true,
                "enforceNodePermissionFlag": true,
                "allowWorkerThreads": true,
                "allowNativeAddons": true,
                "allowChildProcess": false,
                "allowNetworkInUnitTests": false
            },
            "gates": {
                "typesAndLint": "npx tsc --noEmit && npm run lint",
                "vitestEntryRelativePath": "./node_modules/vitest/vitest.mjs",
                "vitestReportRelativePath": "./coverage/alexandria-vitest-report.json",
                "build": "npm run build",
                "minCoveragePercent": 90,
                "skipCoverageForExtensions": [".css", ".scss", ".d.ts", ".md"]
            }
        }
    });

    let config_file_path = target_root.join(".alexandria.json");
    let serialized_config = serde_json::to_string_pretty(&config_json)
        .map_err(|e| format!("Error serializando .alexandria.json: {}", e))?;
    fs::write(&config_file_path, serialized_config)
        .map_err(|e| format!("Error escribiendo .alexandria.json: {}", e))?;

    let gitignore_path = target_root.join(".gitignore");
    let required_ignores = [
        ".env",
        ".env.*",
        "!.env.example",
        "*.db",
        "*.db-wal",
        "*.db-shm",
        ".alexandria/",
        "alexandria-vitest-report.json",
    ];
    let existing_gitignore = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let mut updated_gitignore = existing_gitignore.clone();
    for entry in required_ignores {
        if !existing_gitignore.lines().any(|line| line.trim() == entry) {
            if !updated_gitignore.is_empty() && !updated_gitignore.ends_with('\n') {
                updated_gitignore.push('\n');
            }
            updated_gitignore.push_str(entry);
            updated_gitignore.push('\n');
        }
    }
    if updated_gitignore != existing_gitignore || !gitignore_path.exists() {
        fs::write(&gitignore_path, updated_gitignore)
            .map_err(|e| format!("Error actualizando .gitignore: {}", e))?;
    }

    Ok(config_file_path)
}

pub fn resolve_default_vault_db_path() -> Result<PathBuf, String> {
    let home_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    let alexandria_dir = home_dir.join(".alexandria");
    fs::create_dir_all(&alexandria_dir)
        .map_err(|e| format!("No se pudo crear el directorio {:?}: {}", alexandria_dir, e))?;

    Ok(alexandria_dir.join("vault.db"))
}

pub fn execute_autonomous_run(workspace_root: &Path, instruction: &str) -> Result<String, String> {
    let lower = instruction.to_lowercase();
    let enforced_path = if lower.contains("schema.prisma")
        || lower.contains("auth")
        || lower.contains("adr")
        || lower.contains("arquitectura")
    {
        "omega_pantheon"
    } else {
        "alpha_fast"
    };

    let vault_candidate = if workspace_root.join("alexandria").exists() {
        workspace_root.join("alexandria")
    } else {
        workspace_root.join("wiki")
    };

    let summary = json!({
        "engine": VERSION_BANNER,
        "enforcedPath": enforced_path,
        "instruction": instruction,
        "vaultResolved": vault_candidate.to_string_lossy()
    });

    serde_json::to_string(&summary).map_err(|e| e.to_string())
}
