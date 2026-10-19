use crate::collector::skill_etl::{collect_and_refactor_directory, export_refactored_json_bundle};
use crate::storage::actor::VaultStorageHandle;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub const VERSION_BANNER: &str = "alexandria-runtime v2.4.0-OmniPragmatic";

pub const HELP_BANNER: &str = "\
alexandria-runtime v2.4.0-OmniPragmatic
Headless Deterministic Agent Harness & Kernel-Space Arbitrator

USAGE:
  alexandria <COMMAND> [OPTIONS]

GLOBAL OPTIONS:
  -v, --version                  Print the exact kernel version and exit.
  -h, --help                     Display this help catalog and exit.

COMMANDS:
  init [--vault <path>] [--profile <profile>]
                                 Initialize .alexandria.json, the knowledge vault
                                 (wiki/index.md, log.md, failures.md, architecture/,
                                 modules/, decisions/), Pantheon agent profiles
                                 (.agents/agents/{bel,enki,utu,nabu}.md), and .gitignore.
  collect <source_dir> [--out <output_dir>]
                                 Scan <source_dir>, refactor skills/hooks into canonical
                                 AlexandriaCapsuleJson files in <output_dir> (default:
                                 ./.alexandria/capsules), and synchronize into vault.db.
  mcp                            Start the SQLite WAL actor on ~/.alexandria/vault.db
                                 and serve the MCP JSON-RPC 2.0 server over stdio.
  run \"<directive>\"              Execute a directive in autonomous CLI mode by evaluating
                                 the execution path (Path Alpha / Path Omega).";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Version,
    Help,
    Init {
        vault_path: Option<String>,
        profile: Option<String>,
    },
    Collect {
        source_dir: String,
        output_dir: Option<String>,
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
                        "Flag '--vault' requires a target path argument.".to_string()
                    })?;
                    vault_path = Some(val);
                } else if let Some(val) = arg.strip_prefix("--vault=") {
                    vault_path = Some(val.to_string());
                } else if arg == "--profile" {
                    let val = iter.next().ok_or_else(|| {
                        "Flag '--profile' requires a profile name argument.".to_string()
                    })?;
                    profile = Some(val);
                } else if let Some(val) = arg.strip_prefix("--profile=") {
                    profile = Some(val.to_string());
                } else if arg == "--help" || arg == "-h" {
                    return Ok(CliCommand::Help);
                } else if !arg.starts_with('-') && vault_path.is_none() {
                    vault_path = Some(arg);
                } else {
                    return Err(format!("Unknown option in 'init': '{}'", arg));
                }
            }

            Ok(CliCommand::Init {
                vault_path,
                profile,
            })
        }
        Some("collect") => {
            let mut source_dir: Option<String> = None;
            let mut output_dir: Option<String> = None;

            while let Some(arg) = iter.next() {
                if arg == "--out" || arg == "-o" {
                    let val = iter.next().ok_or_else(|| {
                        "Flag '--out' requires a target directory argument.".to_string()
                    })?;
                    output_dir = Some(val);
                } else if let Some(val) = arg.strip_prefix("--out=") {
                    output_dir = Some(val.to_string());
                } else if arg == "--help" || arg == "-h" {
                    return Ok(CliCommand::Help);
                } else if !arg.starts_with('-') && source_dir.is_none() {
                    source_dir = Some(arg);
                } else {
                    return Err(format!("Unknown option in 'collect': '{}'", arg));
                }
            }

            let src = source_dir.ok_or_else(|| {
                "Command 'collect' requires a <source_dir> argument. Usage: alexandria collect <source_dir> [--out <output_dir>]".to_string()
            })?;

            Ok(CliCommand::Collect {
                source_dir: src,
                output_dir,
            })
        }
        Some("mcp") | Some("serve") => Ok(CliCommand::McpStdio),
        Some("run") => {
            let parts: Vec<String> = iter.collect();
            if parts.is_empty() || parts.iter().all(|s| s.trim().is_empty()) {
                return Err("Command 'run' requires an execution directive.".into());
            }
            Ok(CliCommand::Run {
                instruction: parts.join(" "),
            })
        }
        Some(other) => Err(format!(
            "Unknown subcommand '{}'. Usage: alexandria <init|collect|mcp|run|--version|--help>",
            other
        )),
        None => Err("Usage: alexandria <init|collect|mcp|run|--version|--help>".into()),
    }
}

pub fn execute_init_scaffolding(
    target_root: &Path,
    custom_vault: Option<&str>,
    profile: Option<&str>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(target_root)
        .map_err(|e| format!("Failed to create target root {:?}: {}", target_root, e))?;

    let configured_vault_str = custom_vault.unwrap_or("./wiki");
    let configured_profile = profile.unwrap_or("mvp-fullstack-ts");

    let wiki_dir = target_root.join("wiki");
    for subdir in ["architecture", "modules", "decisions", "raw"] {
        fs::create_dir_all(wiki_dir.join(subdir))
            .map_err(|e| format!("Failed to create subdirectory wiki/{}: {}", subdir, e))?;
    }

    let index_path = wiki_dir.join("index.md");
    if !index_path.exists() {
        let index_content = "---\nid: index\ntitle: \"Alexandria Vault — Master Index\"\ntype: index\nstatus: canonical\nversion: \"2.4.0-OmniPragmatic\"\n---\n\n# Alexandria Vault — Master Index\n\n- Canonical Operational Log: [[log]]\n- Forensic Incident Ledger: [[failures]]\n";
        fs::write(&index_path, index_content)
            .map_err(|e| format!("Failed to write wiki/index.md: {}", e))?;
    }

    let log_path = wiki_dir.join("log.md");
    if !log_path.exists() {
        let log_content = "---\nid: log\ntitle: \"Canonical Operational Log of Approved Changes\"\ntype: operational-log\nstatus: canonical\n---\n\n# Canonical Operational Log of Approved Changes\n\n- Master Index: [[index]]\n";
        fs::write(&log_path, log_content)
            .map_err(|e| format!("Failed to write wiki/log.md: {}", e))?;
    }

    let failures_path = wiki_dir.join("failures.md");
    if !failures_path.exists() {
        let failures_content = "---\nid: failures\ntitle: \"Forensic Incident Ledger and Circuit Breaker\"\ntype: forensic-registry\nstatus: canonical\n---\n\n# Forensic Incident Ledger (`FAIL-XXX`)\n\n- Master Index: [[index]]\n";
        fs::write(&failures_path, failures_content)
            .map_err(|e| format!("Failed to write wiki/failures.md: {}", e))?;
    }

    // Generate Pantheon agent profiles (.agents/agents/{bel,enki,utu,nabu}.md)
    let agents_dir = target_root.join(".agents").join("agents");
    fs::create_dir_all(&agents_dir)
        .map_err(|e| format!("Failed to create .agents/agents directory: {}", e))?;

    let pantheon_profiles: [(&str, &str); 4] = [
        (
            "bel.md",
            "---\nname: bel\ndisplay_name: \"Bel (The Overseer)\"\nrole: \"Zero-Turn Classifier, Topological Evaluator & Sub-Session Dispatcher\"\nmax_subagents: 5\nauthorized_tools:\n  - prepare_task_context\n  - spawn_isolated_worktrees\n  - resolve_skill_capsule\n---\n\n# Bel (The Overseer)\n\nEvaluates incoming instructions in Zero-Turn mode, resolves depth=1 topological context with transitive ADR hoisting, and dispatches bounded parallel worktrees (`max_subagents: 5`). Never modifies `src/` directly.\n",
        ),
        (
            "enki.md",
            "---\nname: enki\ndisplay_name: \"Enki (The Builder)\"\nrole: \"Polymorphic Constructor via Prefix-Pinned Sub-Sessions (DB, Backend, UI)\"\nmax_subagents: 3\nauthorized_tools:\n  - prepare_task_context\n  - resolve_skill_capsule\n  - verify_and_gate_changes\n---\n\n# Enki (The Builder)\n\nExecutes strict RED -> GREEN -> REFACTOR TDD cycles (`max_subagents: 3`) with zero `any`, zero `as unknown as T`, and strict lexical closure inside `prisma.$transaction`.\n",
        ),
        (
            "utu.md",
            "---\nname: utu\ndisplay_name: \"Utu (The Critic)\"\nrole: \"Stateless Cold Auditor for Gate 4 Verification\"\nmax_subagents: 1\nauthorized_tools:\n  - verify_and_gate_changes\n---\n\n# Utu (The Critic)\n\nPerforms impartial, stateless cold-context audits against hoisted `[[ADR-XXX]]` records, `git diff`, and Vitest JSON reports (`max_subagents: 1`).\n",
        ),
        (
            "nabu.md",
            "---\nname: nabu\ndisplay_name: \"Nabu (The Chronicler)\"\nrole: \"Custodian of the Knowledge Vault, Forensic Ledger & Operational Log\"\nmax_subagents: 1\nauthorized_tools:\n  - upsert_architectural_contract\n  - finalize_task_and_log\n---\n\n# Nabu (The Chronicler)\n\nMaintains `wiki/` contracts, `[[ADR-XXX]]` decisions, `FAIL-XXX` forensic records, and the append-only `wiki/log.md` ledger (`max_subagents: 1`).\n",
        ),
    ];

    for (filename, content) in pantheon_profiles {
        let profile_path = agents_dir.join(filename);
        if !profile_path.exists() {
            fs::write(&profile_path, content)
                .map_err(|e| format!("Failed to write .agents/agents/{}: {}", filename, e))?;
        }
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
                "wiki/decisions/**"
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
        .map_err(|e| format!("Failed to serialize .alexandria.json: {}", e))?;
    fs::write(&config_file_path, serialized_config)
        .map_err(|e| format!("Failed to write .alexandria.json: {}", e))?;

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
            .map_err(|e| format!("Failed to update .gitignore: {}", e))?;
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
        .map_err(|e| format!("Failed to create directory {:?}: {}", alexandria_dir, e))?;

    Ok(alexandria_dir.join("vault.db"))
}

pub fn execute_autonomous_run(workspace_root: &Path, instruction: &str) -> Result<String, String> {
    let lower = instruction.to_lowercase();
    let enforced_path = if lower.contains("schema.prisma")
        || lower.contains("auth")
        || lower.contains("adr")
        || lower.contains("architecture")
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

pub async fn execute_collect_subcommand(
    workspace_root: &Path,
    source_dir: &str,
    output_dir: Option<&str>,
    storage: Option<&VaultStorageHandle>,
) -> Result<String, String> {
    let source_path = if Path::new(source_dir).is_absolute() {
        PathBuf::from(source_dir)
    } else {
        workspace_root.join(source_dir)
    };

    let out_path = match output_dir {
        Some(out_str) if Path::new(out_str).is_absolute() => PathBuf::from(out_str),
        Some(out_str) => workspace_root.join(out_str),
        None => workspace_root.join(".alexandria").join("capsules"),
    };

    let capsules = collect_and_refactor_directory(&source_path)?;
    let manifest_path = export_refactored_json_bundle(&capsules, &out_path)?;

    if let Some(storage_handle) = storage {
        for cap in &capsules {
            storage_handle.upsert_skill_capsule(cap).await?;
        }
    }

    let skill_ids: Vec<String> = capsules.iter().map(|c| c.skill_id.clone()).collect();
    let summary = json!({
        "engine": VERSION_BANNER,
        "status": "collected",
        "sourceDir": source_path.to_string_lossy(),
        "outputDir": out_path.to_string_lossy(),
        "manifestPath": manifest_path.to_string_lossy(),
        "totalCapsules": capsules.len(),
        "skillIds": skill_ids
    });

    serde_json::to_string(&summary).map_err(|e| e.to_string())
}

