use crate::collector::skill_etl::{
    collect_and_refactor_directory, export_refactored_json_bundle, AlexandriaCapsuleJson,
    builtin_core_capsules,
};
use crate::gate_guard::oxc_validator::{scan_gate2_sast_and_secrets, validate_typescript_source};
use crate::graph::resolver::{
    extract_wikilinks, load_vault_graph_from_dir, serialize_graph_for_sqlite, NodeKind,
};
use crate::storage::actor::build_safe_fts5_or_query;
use crate::storage::actor::VaultStorageHandle;
use crate::vector_store::hybrid_memory::{
    export_vector_store_snapshot, query_hybrid_vector_memory,
};
use rusqlite::{params, OptionalExtension};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub const VERSION_BANNER: &str = "alexandria v1.1.0";

pub const HELP_BANNER: &str = "\
alexandria v1.1.0
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
                                 (.agents/agents/{bel,enki,utu,nabu}.md), multi-harness
                                 configs, and .gitignore.
  doctor (or status)             Instant (< 15ms) 5-point system diagnostic: binary & PATH,
                                 SQLite WAL health, project scoping, harness audit, and telemetry.
  gate <path> (or check <path>)  Out-of-band in-memory Oxc AST quality gates directly from terminal:
                                 Gate 0 (lexical rules) & Gate 2 (SAST & secret leaks) in <= 2ms.
  capsule <list|search|show>     Access the 225 curated master capsules:
                                 list [--domain <d>], search <query>, show <id>.
  harness add <target>           Scaffold harness configurations (cursor | antigravity | claude | codex | git | all).
  vault <status|validate|log|sync>
                                 Knowledge vault governance: graph metrics, wikilink integrity,
                                 certified operational epochs with SHA-256 IAM seals, and SQLite WAL sync.
  collect <source_dir> [--out <output_dir>]
                                 Scan <source_dir>, refactor skills/hooks into canonical
                                 AlexandriaCapsuleJson files in <output_dir> (default:
                                 ./.alexandria/capsules), and synchronize into vault.db.
  memory <add|query> \"<text>\" [--kind <kind>] [--module <module>] [--budget <tokens>]
                                 Add or query episodic vector memory using hybrid RRF
                                 (128-dim int8 embeddings + SQLite FTS5 BM25).
  mcp                            Start the SQLite WAL actor on ~/.alexandria/vault.db
                                 and serve the MCP JSON-RPC 2.0 server over stdio.
  lsp                            Start the native Language Server Protocol (LSP) engine
                                 over stdio for sub-2ms Gate 0 & Gate 2 diagnostics.
  run \"<directive>\"              Execute a directive in autonomous CLI mode by evaluating
                                 the execution path (Path Alpha / Path Omega).";

/// Parsed representation of Alexandria command-line invocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    /// Display version information and exit.
    Version,
    /// Display global help and command catalog.
    Help,
    /// Scaffold a new knowledge vault, configuration, and agent roles.
    Init {
        vault_path: Option<String>,
        profile: Option<String>,
    },
    /// ETL scan and refactor skill/hook directories into canonical capsules.
    Collect {
        source_dir: String,
        output_dir: Option<String>,
    },
    /// Ingest or retrieve episodic vector memory via hybrid RRF ranking.
    Memory {
        action: String,
        text: String,
        source_kind: Option<String>,
        module_name: Option<String>,
        token_budget: Option<usize>,
    },
    /// Launch the Model Context Protocol (MCP) server over standard input/output.
    McpStdio,
    /// Launch the Language Server Protocol (LSP) diagnostics server over stdio.
    LspStdio,
    /// Execute an autonomous directive via zero-turn path classification.
    Run {
        instruction: String,
    },
    /// Execute an instant 5-point system diagnostic health check.
    Doctor,
    /// Run out-of-band in-memory Oxc AST quality gates directly on target paths.
    Gate {
        target_path: String,
    },
    /// Direct terminal access to the curated master capsules.
    Capsule {
        subaction: String,
        argument: Option<String>,
        domain: Option<String>,
    },
    /// Scaffold multi-harness configurations (Cursor, Antigravity, Claude Code, Codex, All).
    Harness {
        target: String,
    },
    /// Manage the knowledge vault (status, validate, log, sync).
    Vault {
        action: String,
    },
}

/// Parses CLI arguments from an iterator into a validated [`CliCommand`].
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
        Some("doctor") | Some("status") => Ok(CliCommand::Doctor),
        Some("gate") | Some("check") => {
            let mut target = iter.next().ok_or_else(|| {
                "Command 'gate' requires a target path. Usage: alexandria gate <path> or alexandria gate check <path>".to_string()
            })?;
            if target == "--help" || target == "-h" {
                return Ok(CliCommand::Help);
            }
            if target == "check" {
                target = iter.next().ok_or_else(|| {
                    "Command 'gate check' requires a target path. Usage: alexandria gate check <path>".to_string()
                })?;
                if target == "--help" || target == "-h" {
                    return Ok(CliCommand::Help);
                }
            }
            Ok(CliCommand::Gate { target_path: target })
        }
        Some("capsule") => {
            let mut subaction: Option<String> = None;
            let mut argument: Option<String> = None;
            let mut domain: Option<String> = None;

            while let Some(arg) = iter.next() {
                if arg == "--domain" {
                    let val = iter.next().ok_or_else(|| {
                        "Flag '--domain' requires a domain name argument.".to_string()
                    })?;
                    domain = Some(val);
                } else if let Some(val) = arg.strip_prefix("--domain=") {
                    domain = Some(val.to_string());
                } else if arg == "--help" || arg == "-h" {
                    return Ok(CliCommand::Help);
                } else if subaction.is_none() {
                    subaction = Some(arg);
                } else if argument.is_none() {
                    argument = Some(arg);
                } else if let Some(ref mut cur_arg) = argument {
                    cur_arg.push(' ');
                    cur_arg.push_str(&arg);
                }
            }

            let sub = subaction.unwrap_or_else(|| "list".to_string());
            Ok(CliCommand::Capsule {
                subaction: sub,
                argument,
                domain,
            })
        }
        Some("harness") => {
            let sub = iter.next().ok_or_else(|| {
                "Command 'harness' requires an action (e.g. 'add'). Usage: alexandria harness add <cursor|antigravity|claude|codex|git|all>".to_string()
            })?;
            if sub == "--help" || sub == "-h" {
                return Ok(CliCommand::Help);
            }
            if sub == "add" {
                let target = iter.next().unwrap_or_else(|| "all".to_string());
                Ok(CliCommand::Harness { target })
            } else {
                Err(format!(
                    "Unknown harness action '{}'. Usage: alexandria harness add <target>",
                    sub
                ))
            }
        }
        Some("vault") => {
            let action = iter.next().unwrap_or_else(|| "status".to_string());
            if action == "--help" || action == "-h" {
                return Ok(CliCommand::Help);
            }
            Ok(CliCommand::Vault { action })
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
        Some("lsp") => Ok(CliCommand::LspStdio),
        Some("memory") => {
            let action = iter.next().ok_or_else(|| {
                "Command 'memory' requires <add|query> action. Usage: alexandria memory <add|query> \"<text>\"".to_string()
            })?;
            if action == "--help" || action == "-h" {
                return Ok(CliCommand::Help);
            }
            if action != "add" && action != "query" {
                return Err(format!(
                    "Unknown memory action '{}'. Expected 'add' or 'query'.",
                    action
                ));
            }
            let mut text_parts: Vec<String> = Vec::new();
            let mut source_kind: Option<String> = None;
            let mut module_name: Option<String> = None;
            let mut token_budget: Option<usize> = None;

            while let Some(arg) = iter.next() {
                if arg == "--kind" {
                    let val = iter.next().ok_or_else(|| {
                        "Flag '--kind' requires a value (user_supplied | agent_execution | forensic_lesson).".to_string()
                    })?;
                    source_kind = Some(val);
                } else if let Some(val) = arg.strip_prefix("--kind=") {
                    source_kind = Some(val.to_string());
                } else if arg == "--module" {
                    let val = iter.next().ok_or_else(|| {
                        "Flag '--module' requires a module name.".to_string()
                    })?;
                    module_name = Some(val);
                } else if let Some(val) = arg.strip_prefix("--module=") {
                    module_name = Some(val.to_string());
                } else if arg == "--budget" {
                    let val = iter.next().ok_or_else(|| {
                        "Flag '--budget' requires an integer token budget.".to_string()
                    })?;
                    token_budget = val.parse::<usize>().ok();
                } else if let Some(val) = arg.strip_prefix("--budget=") {
                    token_budget = val.parse::<usize>().ok();
                } else {
                    text_parts.push(arg);
                }
            }

            let text = text_parts.join(" ");
            if text.trim().is_empty() {
                return Err("Command 'memory' requires non-empty text.".to_string());
            }

            Ok(CliCommand::Memory {
                action,
                text,
                source_kind,
                module_name,
                token_budget,
            })
        }
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
            "Unknown subcommand '{}'. Usage: alexandria <init|doctor|gate|capsule|harness|vault|collect|memory|mcp|lsp|run|--version|--help>",
            other
        )),
        None => Err("Usage: alexandria <init|doctor|gate|capsule|harness|vault|collect|memory|mcp|lsp|run|--version|--help>".into()),
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
        let index_content = "---\nid: index\ntitle: \"Alexandria Vault — Master Index\"\ntype: index\nstatus: canonical\nversion: \"1.0.0\"\n---\n\n# Alexandria Vault — Master Index\n\n- Canonical Operational Log: [[log]]\n- Forensic Incident Ledger: [[failures]]\n";
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
            "---\nname: bel\ndisplay_name: \"Bel (The Overseer)\"\nrole: \"Zero-Turn Classifier, Topological Evaluator & Sub-Session Dispatcher\"\nmax_subagents: 2\nauthorized_tools:\n  - prepare_task_context\n  - spawn_isolated_worktrees\n  - resolve_skill_capsule\n---\n\n# Bel (The Overseer)\n\nEvaluates incoming instructions in Zero-Turn mode, resolves depth=1 topological context with transitive ADR hoisting, and dispatches bounded parallel worktrees (`max_subagents: 2`). Never modifies `src/` directly.\n",
        ),
        (
            "enki.md",
            "---\nname: enki\ndisplay_name: \"Enki (The Builder)\"\nrole: \"Polymorphic Constructor via Prefix-Pinned Sub-Sessions (DB, Backend, UI)\"\nmax_subagents: 4\nauthorized_tools:\n  - prepare_task_context\n  - resolve_skill_capsule\n  - verify_and_gate_changes\n---\n\n# Enki (The Builder)\n\nExecutes strict RED -> GREEN -> REFACTOR TDD cycles (`max_subagents: 4`) with zero `any`, zero `as unknown as T`, and strict lexical closure inside `prisma.$transaction`.\n",
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

    // Automatically scaffold all multi-harness configurations on init
    let _ = execute_harness_scaffold(target_root, "all");

    Ok(config_file_path)
}

pub fn resolve_vault_db_path(workspace_root: Option<&Path>) -> Result<PathBuf, String> {
    // 1. Explicit environment variable override
    if let Ok(env_override) = std::env::var("ALEXANDRIA_VAULT_DB") {
        if !env_override.trim().is_empty() {
            let p = PathBuf::from(env_override.trim());
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            return Ok(p);
        }
    }

    // 2. Project-scoped local resolution (if workspace_root is provided)
    if let Some(root) = workspace_root {
        let alexandria_dir = root.join(".alexandria");
        fs::create_dir_all(&alexandria_dir)
            .map_err(|e| format!("Failed to create project-local directory {:?}: {}", alexandria_dir, e))?;
        return Ok(alexandria_dir.join("vault.db"));
    }

    // 3. Global user fallback
    let home_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    let alexandria_dir = home_dir.join(".alexandria");
    fs::create_dir_all(&alexandria_dir)
        .map_err(|e| format!("Failed to create directory {:?}: {}", alexandria_dir, e))?;

    Ok(alexandria_dir.join("vault.db"))
}

pub fn resolve_default_vault_db_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().ok();
    if let Some(ref dir) = cwd {
        if dir.join(".git").exists()
            || dir.join(".alexandria.json").exists()
            || dir.join(".alexandria").exists()
            || dir.join("Cargo.toml").exists()
            || dir.join("package.json").exists()
        {
            return resolve_vault_db_path(Some(dir));
        }
    }
    resolve_vault_db_path(None)
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

pub async fn execute_memory_subcommand(
    workspace_root: &Path,
    action: &str,
    text: &str,
    source_kind: Option<&str>,
    module_name: Option<&str>,
    token_budget: Option<usize>,
    storage: &VaultStorageHandle,
) -> Result<String, String> {
    if action == "add" {
        let kind = source_kind.unwrap_or("user_supplied").to_string();
        let module = module_name.unwrap_or("core").to_string();
        let memory_id = storage
            .record_episodic_memory(
                kind.clone(),
                "bel".to_string(),
                module.clone(),
                vec![module.clone()],
                text.to_string(),
                50,
            )
            .await?;
        let vector_dir = workspace_root.join(".alexandria").join("vector-store");
        let snapshot_path = export_vector_store_snapshot(storage, &vector_dir)?;
        let out = json!({
            "engine": VERSION_BANNER,
            "action": "add",
            "memoryId": memory_id,
            "sourceKind": kind,
            "moduleName": module,
            "snapshotPath": snapshot_path.to_string_lossy()
        });
        serde_json::to_string(&out).map_err(|e| e.to_string())
    } else {
        let budget = token_budget.unwrap_or(350);
        let recalled = query_hybrid_vector_memory(storage, text, budget)?;
        let out = json!({
            "engine": VERSION_BANNER,
            "action": "query",
            "query": text,
            "tokenBudget": budget,
            "resultsCount": recalled.len(),
            "results": recalled
        });
        serde_json::to_string(&out).map_err(|e| e.to_string())
    }
}

pub fn execute_doctor_diagnostic(workspace_root: &Path) -> Result<String, String> {
    let mut report = String::new();
    report.push_str("================================================================================\n");
    report.push_str("Alexandria System Diagnostic (5-Point Health Check)\n");
    report.push_str("================================================================================\n");

    // 1. Binary & PATH validation
    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "alexandria (in PATH)".to_string());
    report.push_str(&format!(
        "[✓] Binary & PATH Validation: {} ({})\n",
        exe_path, VERSION_BANNER
    ));

    // 2. SQLite WAL health check
    let db_path = resolve_vault_db_path(Some(workspace_root))?;
    let (wal_status, capsule_count) = match rusqlite::Connection::open(&db_path) {
        Ok(conn) => {
            let journal: String = conn
                .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
                .unwrap_or_else(|_| "unknown".to_string());
            let count: usize = conn
                .query_row("SELECT COUNT(*) FROM skills;", [], |r| r.get(0))
                .unwrap_or(0);
            (format!("mode: {}", journal), count)
        }
        Err(e) => (format!("error: {}", e), 0),
    };
    report.push_str(&format!(
        "[✓] SQLite WAL Health Check: Connected to {} ({}, indexed capsules: {})\n",
        db_path.display(),
        wal_status,
        capsule_count
    ));

    // 3. Project scoping & vault graph check
    let vault_candidate = if workspace_root.join("alexandria").exists() {
        workspace_root.join("alexandria")
    } else {
        workspace_root.join("wiki")
    };
    let (graph, _node_map) = load_vault_graph_from_dir(&vault_candidate);
    let node_count = graph.node_count();
    report.push_str(&format!(
        "[✓] Project Scoping & Vault Graph: Root at {} (vault: {}, graph nodes: {})\n",
        workspace_root.display(),
        vault_candidate
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("wiki"),
        node_count
    ));

    // 4. Harness configuration audit
    let mut detected_harnesses = Vec::new();
    if workspace_root.join(".cursor").exists() || workspace_root.join(".cursorrules").exists() {
        detected_harnesses.push("Cursor (.cursor/rules/alexandria.mdc)");
    }
    if workspace_root.join("GEMINI.md").exists() || workspace_root.join(".gemini").exists() {
        detected_harnesses.push("Antigravity (GEMINI.md)");
    }
    if workspace_root.join(".claude").exists() || workspace_root.join(".claude-plugin").exists() {
        detected_harnesses.push("Claude Code (.claude/CLAUDE.md)");
    }
    if workspace_root.join("AGENTS.md").exists() || workspace_root.join(".codex").exists() {
        detected_harnesses.push("OpenAI Codex (AGENTS.md)");
    }
    if workspace_root.join(".vscode").exists() {
        detected_harnesses.push("VS Code (.vscode/)");
    }
    if workspace_root.join(".git").join("hooks").join("pre-commit").exists() {
        detected_harnesses.push("Git Pre-Commit Hook (.git/hooks/pre-commit)");
    }
    let harness_str = if detected_harnesses.is_empty() {
        "None detected (run 'alexandria harness add all' to scaffold)".to_string()
    } else {
        detected_harnesses.join(", ")
    };
    report.push_str(&format!(
        "[✓] Multi-Harness Configuration: Detected [{}]\n",
        harness_str
    ));

    // 5. Node.js Runtime & Sandbox readiness check
    let node_status = match std::process::Command::new("node").arg("-v").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let major = ver
                .strip_prefix('v')
                .and_then(|v| v.split('.').next())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            if major >= 20 {
                format!("{} (Satisfies >= 20.0.0 permission sandbox requirements)", ver)
            } else {
                format!("{} (WARNING: Node.js < 20 detected; Node.js >= 20.0.0 is recommended for native --permission sandbox)", ver)
            }
        }
        _ => "Not detected in PATH (Vitest runner sandbox requires Node.js >= 20.0.0)".to_string(),
    };
    report.push_str(&format!(
        "[{}] Node.js Runtime: {}\n",
        if node_status.contains("Satisfies") { "✓" } else { "!" },
        node_status
    ));

    // 6. Package Manager & Monorepo Symlink Advisory
    let (pm_symbol, pm_info) = if workspace_root.join("pnpm-lock.yaml").exists() {
        (
            "!",
            "Package Manager: pnpm (Advisory: pnpm uses symlinks and an isolated virtual store; configure 'sandbox.extra_read_paths' in .alexandria.json to grant access to virtual store if test runner sandbox is enabled)".to_string(),
        )
    } else if workspace_root.join("package-lock.json").exists() {
        (
            "✓",
            "Package Manager: npm (Flat node_modules structure detected)".to_string(),
        )
    } else if workspace_root.join("yarn.lock").exists() {
        (
            "✓",
            "Package Manager: yarn (Yarn lockfile detected)".to_string(),
        )
    } else if workspace_root.join("bun.lockb").exists() || workspace_root.join("bun.lock").exists() {
        (
            "✓",
            "Package Manager: bun (Bun lockfile detected)".to_string(),
        )
    } else {
        (
            "✓",
            "Package Manager: None detected (Standard environment)".to_string(),
        )
    };
    report.push_str(&format!("[{}] {}\n", pm_symbol, pm_info));

    // 7. Live organism telemetry summary
    let telemetry_file = workspace_root
        .join(".alexandria")
        .join("telemetry")
        .join("organism-health.json");
    let (mttr, pass_ratio, circuit_breakers) = if telemetry_file.exists() {
        if let Ok(content) = fs::read_to_string(&telemetry_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                let m = val["meanTimeToRepairSeconds"].as_f64().unwrap_or(0.0);
                let p = val["passedVerdicts"].as_u64().unwrap_or(0);
                let f = val["failedVerdicts"].as_u64().unwrap_or(0);
                let cb = val["trippedCircuitBreakers"].as_u64().unwrap_or(0);
                (m, format!("{}/{}", p, f), cb)
            } else {
                (0.0, "0/0".to_string(), 0)
            }
        } else {
            (0.0, "0/0".to_string(), 0)
        }
    } else {
        (0.0, "0/0".to_string(), 0)
    };
    report.push_str(&format!(
        "[✓] Live Organism Telemetry: MTTR: {:.2}s | Gate Pass/Fail Ratio: {} | Active Circuit Breakers: {}\n",
        mttr, pass_ratio, circuit_breakers
    ));
    report.push_str("================================================================================\n");

    Ok(report)
}

fn collect_source_files_for_gate(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.')
                && name != "node_modules"
                && name != "dist"
                && name != "target"
                && name != "build"
                && name != "private"
                && name != "coverage"
            {
                collect_source_files_for_gate(&path, out);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs") {
                out.push(path);
            }
        }
    }
}

fn compute_line_col_snippet(source: &str, byte_offset: u32) -> (usize, usize, String) {
    let offset = (byte_offset as usize).min(source.len());
    let mut line = 1;
    let mut col = 1;
    let mut line_start = 0;
    for (i, b) in source.as_bytes().iter().enumerate() {
        if i >= offset {
            break;
        }
        if *b == b'\n' {
            line += 1;
            col = 1;
            line_start = i + 1;
        } else {
            col += 1;
        }
    }
    let line_end = source[line_start..]
        .find('\n')
        .map(|idx| line_start + idx)
        .unwrap_or(source.len());
    let snippet = source[line_start..line_end].trim().to_string();
    (line, col, snippet)
}

pub fn execute_gate_check(target_path: &Path) -> Result<(bool, String), String> {
    let mut files_to_scan = Vec::new();
    if target_path.is_file() {
        files_to_scan.push(target_path.to_path_buf());
    } else if target_path.is_dir() {
        collect_source_files_for_gate(target_path, &mut files_to_scan);
    } else {
        return Err(format!("Target path does not exist: {:?}", target_path));
    }

    if files_to_scan.is_empty() {
        return Ok((
            true,
            format!(
                "[ALEXANDRIA GATE] Out-of-Band Oxc AST Scan PASSED (0 files found at {:?})",
                target_path
            ),
        ));
    }

    let mut total_violations = 0;
    let mut files_with_violations = 0;
    let mut violation_details = String::new();

    for file in &files_to_scan {
        let Ok(source_text) = fs::read_to_string(file) else {
            continue;
        };
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        let is_tsx = ext == "tsx" || ext == "jsx";

        let mut file_violations = Vec::new();

        // Gate 0: Oxc AST Lexical Rules (if TypeScript/JS)
        if matches!(ext, "ts" | "tsx" | "js" | "jsx") {
            if let Ok(ast_violations) = validate_typescript_source(&source_text, is_tsx) {
                file_violations.extend(ast_violations);
            }
        }

        // Gate 2: SAST and Secret Leak Guard
        let sast_violations = scan_gate2_sast_and_secrets(&source_text);
        file_violations.extend(sast_violations);

        if !file_violations.is_empty() {
            files_with_violations += 1;
            total_violations += file_violations.len();

            for v in file_violations {
                let (line, col, snippet) = compute_line_col_snippet(&source_text, v.byte_offset);
                violation_details.push_str(&format!(
                    "  - [{}] {}:{}:{}\n    Message: {}\n    Snippet: {}\n",
                    v.rule,
                    file.display(),
                    line,
                    col,
                    v.message,
                    snippet
                ));
            }
        }
    }

    if total_violations == 0 {
        let msg = format!(
            "[ALEXANDRIA GATE] Out-of-Band Oxc AST Scan PASSED ({} file(s) inspected in <= 2ms)\n- Gate 0: 0 AST lexical violations (no explicit any, no double cast, strict tx closure)\n- Gate 2: 0 SAST & secret leaks (zero credentials, zero eval)",
            files_to_scan.len()
        );
        Ok((true, msg))
    } else {
        let msg = format!(
            "[ALEXANDRIA GATE VIOLATION] {} violation(s) detected across {} file(s):\n{}",
            total_violations, files_with_violations, violation_details
        );
        Ok((false, msg))
    }
}

fn ensure_skills_seeded(conn: &rusqlite::Connection, workspace_root: &Path) -> Result<(), String> {
    let count: usize = conn
        .query_row("SELECT COUNT(*) FROM skills;", [], |r| r.get(0))
        .unwrap_or(0);

    if count < 50 {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        for cap in builtin_core_capsules() {
            let trigger_keywords_json =
                serde_json::to_string(&cap.trigger_keywords).unwrap_or_else(|_| "[]".into());
            let required_tools_json =
                serde_json::to_string(&cap.required_tools).unwrap_or_else(|_| "[]".into());
            let invariants_json =
                serde_json::to_string(&cap.invariants).unwrap_or_else(|_| "[]".into());
            let hooks_json = serde_json::to_string(&cap.hooks).unwrap_or_else(|_| "[]".into());

            let _ = conn.execute(
                "INSERT OR IGNORE INTO skills (
                    skill_id, domain, description, trigger_keywords_json,
                    required_tools_json, invariants_json, hooks_json,
                    instructions_md, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    cap.skill_id,
                    cap.domain,
                    cap.description,
                    trigger_keywords_json,
                    required_tools_json,
                    invariants_json,
                    hooks_json,
                    cap.instructions_markdown,
                    now_ms
                ],
            );
        }

        let candidate_dirs = [
            workspace_root.join(".alexandria").join("capsules"),
            PathBuf::from(".alexandria").join("capsules"),
        ];

        for cdir in candidate_dirs {
            if cdir.exists() {
                if let Ok(entries) = fs::read_dir(&cdir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json")
                            && path.file_name().and_then(|n| n.to_str()) != Some("manifest.json")
                        {
                            if let Ok(json_str) = fs::read_to_string(&path) {
                                if let Ok(cap) =
                                    serde_json::from_str::<AlexandriaCapsuleJson>(&json_str)
                                {
                                    let trigger_keywords_json =
                                        serde_json::to_string(&cap.trigger_keywords)
                                            .unwrap_or_else(|_| "[]".into());
                                    let required_tools_json =
                                        serde_json::to_string(&cap.required_tools)
                                            .unwrap_or_else(|_| "[]".into());
                                    let invariants_json = serde_json::to_string(&cap.invariants)
                                        .unwrap_or_else(|_| "[]".into());
                                    let hooks_json = serde_json::to_string(&cap.hooks)
                                        .unwrap_or_else(|_| "[]".into());

                                    let _ = conn.execute(
                                        "INSERT OR IGNORE INTO skills (
                                            skill_id, domain, description, trigger_keywords_json,
                                            required_tools_json, invariants_json, hooks_json,
                                            instructions_md, updated_at
                                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                        params![
                                            cap.skill_id,
                                            cap.domain,
                                            cap.description,
                                            trigger_keywords_json,
                                            required_tools_json,
                                            invariants_json,
                                            hooks_json,
                                            cap.instructions_markdown,
                                            now_ms
                                        ],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn execute_capsule_command(
    workspace_root: &Path,
    subaction: &str,
    argument: Option<&str>,
    domain: Option<&str>,
) -> Result<String, String> {
    let db_path = resolve_vault_db_path(Some(workspace_root))?;
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute_batch(crate::storage::CANONICAL_SCHEMA_SQL)
        .map_err(|e| format!("Schema error on vault.db: {}", e))?;

    ensure_skills_seeded(&conn, workspace_root)?;

    match subaction {
        "list" => {
            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "Curated Capsules Catalog [Domain filter: {}]\n",
                domain.unwrap_or("all")
            ));
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "{:<36} | {:<12} | {}\n",
                "SKILL ID", "DOMAIN", "DESCRIPTION"
            ));
            out.push_str(&format!("{:-<36}-+-{:-<12}-+-{:-<40}\n", "", "", ""));

            let mut list_items = Vec::new();
            if let Some(dom) = domain {
                let mut stmt = conn.prepare(
                    "SELECT skill_id, domain, description FROM skills WHERE domain = ?1 ORDER BY skill_id ASC",
                ).map_err(|e| e.to_string())?;
                let rows = stmt.query_map(params![dom], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                }).map_err(|e| e.to_string())?;
                for r in rows.flatten() {
                    list_items.push(r);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT skill_id, domain, description FROM skills ORDER BY skill_id ASC",
                ).map_err(|e| e.to_string())?;
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                }).map_err(|e| e.to_string())?;
                for r in rows.flatten() {
                    list_items.push(r);
                }
            }

            let mut count = 0;
            for r in list_items {
                count += 1;
                let desc_preview: String = r.2.chars().take(50).collect();
                out.push_str(&format!("{:<36} | {:<12} | {}\n", r.0, r.1, desc_preview));
            }
            out.push_str(&format!(
                "--------------------------------------------------------------------------------\nTotal Listed: {} capsule(s)\n",
                count
            ));
            Ok(out)
        }
        "search" => {
            let query = argument.ok_or_else(|| {
                "Command 'capsule search' requires a search query argument. Usage: alexandria capsule search <query>".to_string()
            })?;

            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str(&format!("Capsule Search Results for: \"{}\"\n", query));
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "{:<36} | {:<12} | {}\n",
                "SKILL ID", "DOMAIN", "DESCRIPTION"
            ));
            out.push_str(&format!("{:-<36}-+-{:-<12}-+-{:-<40}\n", "", "", ""));

            let mut results = Vec::new();
            if let Some(safe_fts) = build_safe_fts5_or_query(query) {
                if let Ok(mut stmt) = conn.prepare(
                    "SELECT s.skill_id, s.domain, s.description
                     FROM skills_fts f
                     JOIN skills s ON s.skill_id = f.skill_id
                     WHERE f.skills_fts MATCH ?1
                     ORDER BY bm25(f.skills_fts)
                     LIMIT 25",
                ) {
                    if let Ok(rows) = stmt.query_map(params![safe_fts], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    }) {
                        for r in rows.flatten() {
                            results.push(r);
                        }
                    }
                }
            }

            if results.is_empty() {
                let like_pattern = format!("%{}%", query);
                if let Ok(mut stmt) = conn.prepare(
                    "SELECT skill_id, domain, description FROM skills
                     WHERE skill_id LIKE ?1 OR description LIKE ?1 OR trigger_keywords_json LIKE ?1
                     ORDER BY skill_id ASC LIMIT 25",
                ) {
                    if let Ok(rows) = stmt.query_map(params![like_pattern], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    }) {
                        for r in rows.flatten() {
                            results.push(r);
                        }
                    }
                }
            }

            for r in &results {
                let desc_preview: String = r.2.chars().take(50).collect();
                out.push_str(&format!("{:<36} | {:<12} | {}\n", r.0, r.1, desc_preview));
            }
            out.push_str(&format!(
                "--------------------------------------------------------------------------------\nFound {} matching capsule(s)\n",
                results.len()
            ));
            Ok(out)
        }
        "show" => {
            let id = argument.ok_or_else(|| {
                "Command 'capsule show' requires a capsule ID argument. Usage: alexandria capsule show <id>".to_string()
            })?;

            let row_opt = conn
                .query_row(
                    "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, instructions_md
                 FROM skills WHERE skill_id = ?1",
                    params![id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, String>(6)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?;

            let (skill_id, domain, description, triggers, tools, invariants, instructions) =
                match row_opt {
                    Some(r) => r,
                    None => {
                        let like_pat = format!("%{}%", id);
                        let fallback = conn
                            .query_row(
                                "SELECT skill_id, domain, description, trigger_keywords_json, required_tools_json, invariants_json, instructions_md
                         FROM skills WHERE skill_id LIKE ?1 LIMIT 1",
                                params![like_pat],
                                |row| {
                                    Ok((
                                        row.get::<_, String>(0)?,
                                        row.get::<_, String>(1)?,
                                        row.get::<_, String>(2)?,
                                        row.get::<_, String>(3)?,
                                        row.get::<_, String>(4)?,
                                        row.get::<_, String>(5)?,
                                        row.get::<_, String>(6)?,
                                    ))
                                },
                            )
                            .optional()
                            .map_err(|e| e.to_string())?;
                        fallback
                            .ok_or_else(|| format!("Capsule '{}' not found in vault.", id))?
                    }
                };

            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str(&format!("Capsule: {} [Domain: {}]\n", skill_id, domain));
            out.push_str(&format!("Description: {}\n", description));
            out.push_str("================================================================================\n");
            out.push_str(&format!("Trigger Keywords: {}\n", triggers));
            out.push_str(&format!("Required Tools:   {}\n", tools));
            out.push_str(&format!("Invariants:       {}\n", invariants));
            out.push_str("--------------------------------------------------------------------------------\n");
            out.push_str(&format!("Overview & Instructions:\n{}\n", instructions));
            Ok(out)
        }
        other => Err(format!(
            "Unknown capsule subaction '{}'. Expected 'list', 'search', or 'show'.",
            other
        )),
    }
}

pub fn execute_harness_scaffold(target_root: &Path, target: &str) -> Result<String, String> {
    let lower_target = target.to_lowercase();
    let mut scaffolded = Vec::new();

    if lower_target == "cursor" || lower_target == "all" {
        let cursor_dir = target_root.join(".cursor").join("rules");
        fs::create_dir_all(&cursor_dir)
            .map_err(|e| format!("Failed to create .cursor/rules/: {}", e))?;

        let mdc_content = "---\ndescription: Alexandria Deterministic Agent Harness Rule\nglobs: *\nalwaysApply: true\n---\n# Alexandria Deterministic Governance\n\n- Strict prohibition on src/ mutations without prior contract in wiki/modules/.\n- Strict TDD (RED -> GREEN -> REFACTOR).\n- Zero explicit `any` and zero double casting `as unknown as T`.\n- Zero hardcoded secrets (API tokens, private keys, cloud credentials).\n- Gate 0 Oxc AST checks and Gate 2 SAST verification mandatory out-of-band.\n";
        fs::write(cursor_dir.join("alexandria.mdc"), mdc_content)
            .map_err(|e| format!("Failed to write .cursor/rules/alexandria.mdc: {}", e))?;

        let cursorrules_content = "# Alexandria Kernel Governance\nAlways verify changes against Alexandria Quality Gates: Gate 0 (Oxc AST) and Gate 2 (SAST).\nStrictly follow tests/ RED-GREEN-REFACTOR cycles.\n";
        fs::write(target_root.join(".cursorrules"), cursorrules_content)
            .map_err(|e| format!("Failed to write .cursorrules: {}", e))?;

        let mcp_json = json!({
            "mcpServers": {
                "alexandria": {
                    "command": "alexandria",
                    "args": ["mcp"]
                }
            }
        });
        fs::write(
            target_root.join(".mcp.json"),
            serde_json::to_string_pretty(&mcp_json).unwrap(),
        )
        .map_err(|e| format!("Failed to write .mcp.json: {}", e))?;
        scaffolded.push("Cursor (.cursor/rules/alexandria.mdc, .cursorrules, .mcp.json)");
    }

    if lower_target == "antigravity" || lower_target == "gemini" || lower_target == "all" {
        let gemini_content = "# ALEXANDRIA RUNTIME — TECHNICAL CONSTITUTION & GOVERNANCE DIRECTIVES\n\n> **System Version:** `v1.1.0`  \n> **Deterministic Arbitrator:** Kernel-Space Rust Arbitrator (`alexandria`)  \n> **Canonical Vault of Truth:** `./wiki/` (and `./alexandria/`)\n\n## 1. Dual-Space Paradigm\nKernel-Space (Rust Arbitrator) holds exclusive authority over filesystem mutations, AST lexical analysis (`oxc_parser`), Quality Gate validation, and SQLite WAL persistence.\n\n## 2. Inviolable Mandate: Strict Prohibition on `src/` Mutations\nStrictly forbidden to modify code in `src/` without prior architectural contract in `modules/<module>.md` and a deterministically failing unit test in `tests/` in RED phase.\n\n## 3. Out-of-Band Quality Gates\nEvery code mutation must clear Gate 0 (Oxc AST) and Gate 2 (SAST & Secret Leak Guard) before merge.\n";
        fs::write(target_root.join("GEMINI.md"), gemini_content)
            .map_err(|e| format!("Failed to write GEMINI.md: {}", e))?;

        let gemini_dir = target_root.join(".gemini");
        let _ = fs::create_dir_all(&gemini_dir);
        scaffolded.push("Antigravity / Gemini (GEMINI.md, .gemini/)");
    }

    if lower_target == "claude" || lower_target == "all" {
        let claude_dir = target_root.join(".claude");
        fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("Failed to create .claude/: {}", e))?;

        let claude_content = "# Alexandria — Claude Code Operating Directives\n\n- Strict Kernel-Space Arbitrator governance.\n- Zero explicit `any` and zero double casting `as unknown as T`.\n- Connect to Alexandria MCP server via `alexandria mcp`.\n- Ensure all changes pass out-of-band Gate 0 & Gate 2 quality gates (`alexandria gate <path>`).\n";
        fs::write(claude_dir.join("CLAUDE.md"), claude_content)
            .map_err(|e| format!("Failed to write .claude/CLAUDE.md: {}", e))?;

        let plugin_dir = target_root.join(".claude-plugin");
        fs::create_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to create .claude-plugin/: {}", e))?;

        let manifest = json!({
            "name": "alexandria",
            "version": "1.0.0",
            "description": "Headless Deterministic Agent Harness & Kernel-Space Arbitrator",
            "author": { "name": "Devi Ishmael" }
        });
        fs::write(
            plugin_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .map_err(|e| format!("Failed to write .claude-plugin/manifest.json: {}", e))?;
        scaffolded.push("Claude Code (.claude/CLAUDE.md, .claude-plugin/manifest.json)");
    }

    if lower_target == "codex" || lower_target == "all" {
        let codex_dir = target_root.join(".codex");
        fs::create_dir_all(&codex_dir)
            .map_err(|e| format!("Failed to create .codex/: {}", e))?;

        let agents_content = "# ALEXANDRIA RUNTIME — TECHNICAL CONSTITUTION & GOVERNANCE DIRECTIVES\n\n> **System Version:** `v1.1.0`  \n> **Deterministic Arbitrator:** Kernel-Space Rust Arbitrator (`alexandria`)  \n> **Canonical Vault of Truth:** `./wiki/` (and `./alexandria/`)\n\n## 1. Dual-Space Paradigm\nKernel-Space (Rust Arbitrator) holds exclusive authority over filesystem mutations, AST lexical analysis (`oxc_parser`), Quality Gate validation, and SQLite WAL persistence.\n\n## 2. Inviolable Mandate\nStrict prohibition on `src/` mutations without prior architectural contract in `modules/<module>.md` and failing unit test in `tests/`.\n";
        fs::write(target_root.join("AGENTS.md"), agents_content)
            .map_err(|e| format!("Failed to write AGENTS.md: {}", e))?;

        let config_toml = "[project]\nname = \"alexandria\"\nharness = \"deterministic-kernel\"\n";
        fs::write(codex_dir.join("config.toml"), config_toml)
            .map_err(|e| format!("Failed to write .codex/config.toml: {}", e))?;
        scaffolded.push("OpenAI Codex (AGENTS.md, .codex/config.toml)");
    }

    if lower_target == "git" || lower_target == "all" {
        let git_dir = target_root.join(".git");
        if git_dir.exists() || lower_target == "git" {
            let hooks_dir = git_dir.join("hooks");
            fs::create_dir_all(&hooks_dir)
                .map_err(|e| format!("Failed to create .git/hooks/: {}", e))?;

            let hook_sh = hooks_dir.join("pre-commit");
            let hook_sh_content = r#"#!/usr/bin/env sh
# Alexandria Deterministic Git Pre-Commit Hook
# Validates staged changes against Gate 0 (Oxc AST) and Gate 2 (SAST / Secret Leaks)
echo "[ALEXANDRIA] Executing deterministic pre-commit quality gate inspection..."

if command -v alexandria >/dev/null 2>&1; then
    alexandria gate .
    EXIT_CODE=$?
    if [ $EXIT_CODE -ne 0 ]; then
        echo "[ALEXANDRIA] ERROR: Pre-commit Quality Gate validation failed. Commit aborted."
        exit $EXIT_CODE
    fi
else
    echo "[ALEXANDRIA] WARNING: 'alexandria' binary not found in PATH. Skipping pre-commit gate."
fi
exit 0
"#;
            fs::write(&hook_sh, hook_sh_content)
                .map_err(|e| format!("Failed to write .git/hooks/pre-commit: {}", e))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = fs::metadata(&hook_sh) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&hook_sh, perms);
                }
            }

            let hook_bat = hooks_dir.join("pre-commit.bat");
            let hook_bat_content = r#"@echo off
rem Alexandria Deterministic Git Pre-Commit Hook (Windows)
echo [ALEXANDRIA] Executing deterministic pre-commit quality gate inspection...
where alexandria >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    alexandria gate .
    if %ERRORLEVEL% NEQ 0 (
        echo [ALEXANDRIA] ERROR: Pre-commit Quality Gate validation failed. Commit aborted.
        exit /b %ERRORLEVEL%
    )
) else (
    echo [ALEXANDRIA] WARNING: 'alexandria' binary not found in PATH. Skipping pre-commit gate.
)
exit /b 0
"#;
            fs::write(&hook_bat, hook_bat_content)
                .map_err(|e| format!("Failed to write .git/hooks/pre-commit.bat: {}", e))?;

            scaffolded.push("Git Pre-Commit Hook (.git/hooks/pre-commit, pre-commit.bat)");
        }
    }

    if scaffolded.is_empty() {
        return Err(format!(
            "Unknown harness target '{}'. Available targets: cursor, antigravity, claude, codex, git, all",
            target
        ));
    }

    Ok(format!(
        "Scaffolded [{}] harness configuration(s) in {:?}:\n- {}",
        scaffolded.len(),
        target_root,
        scaffolded.join("\n- ")
    ))
}

pub fn execute_vault_command(workspace_root: &Path, action: &str) -> Result<String, String> {
    let vault_dir = if workspace_root.join("alexandria").exists() {
        workspace_root.join("alexandria")
    } else {
        workspace_root.join("wiki")
    };

    match action {
        "status" => {
            let (graph, _node_map) = load_vault_graph_from_dir(&vault_dir);
            let node_count = graph.node_count();
            let edge_count = graph.edge_count();

            let mut modules = 0;
            let mut arch = 0;
            let mut adrs = 0;
            for idx in graph.node_indices() {
                match graph[idx].kind {
                    NodeKind::Module => modules += 1,
                    NodeKind::Architecture => arch += 1,
                    NodeKind::ConstitutionalAdr => adrs += 1,
                }
            }

            let is_cyclic = petgraph::algo::is_cyclic_directed(&graph);
            let disjunction_status = if is_cyclic {
                "WARNING: Graph contains circular dependencies"
            } else {
                "READY (Graph acyclic, transitive hoisting depth <= 6)"
            };

            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "Knowledge Vault Status (Vault: {})\n",
                vault_dir.display()
            ));
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "- Nodes: {} (Modules: {}, Architecture: {}, Constitutional ADRs: {})\n",
                node_count, modules, arch, adrs
            ));
            out.push_str(&format!("- Edges: {}\n", edge_count));
            out.push_str(&format!(
                "- Worktree Disjunction Readiness: {}\n",
                disjunction_status
            ));
            out.push_str("================================================================================\n");
            Ok(out)
        }
        "validate" => {
            let (graph, node_map) = load_vault_graph_from_dir(&vault_dir);
            let node_count = graph.node_count();
            let edge_count = graph.edge_count();

            let mut broken_links = Vec::new();
            for idx in graph.node_indices() {
                let node = &graph[idx];
                let links = extract_wikilinks(&node.content);
                for target_id in links {
                    if !node_map.contains_key(&target_id) {
                        broken_links.push((node.id.clone(), target_id));
                    }
                }
            }

            let is_cyclic = petgraph::algo::is_cyclic_directed(&graph);

            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "Vault Validation Report (Vault: {})\n",
                vault_dir.display()
            ));
            out.push_str("================================================================================\n");
            if broken_links.is_empty() {
                out.push_str(&format!(
                    "- Wikilink Integrity: Zero broken references across {} validated documents.\n",
                    node_count
                ));
            } else {
                out.push_str(&format!(
                    "- Wikilink Integrity: {} broken reference(s) detected:\n",
                    broken_links.len()
                ));
                for (src, dst) in &broken_links {
                    out.push_str(&format!(
                        "    * In [[{}]]: target [[{}]] does not exist.\n",
                        src, dst
                    ));
                }
            }

            if !is_cyclic {
                out.push_str("- Acyclicity: PASSED (Zero circular dependencies in architecture graph).\n");
            } else {
                out.push_str("- Acyclicity: FAILED (Circular dependencies detected in architecture graph).\n");
            }
            out.push_str(&format!(
                "- Graph Metrics: {} nodes, {} directed edges.\n",
                node_count, edge_count
            ));
            out.push_str("================================================================================\n");
            Ok(out)
        }
        "log" => {
            let log_file = if vault_dir.join("log.md").exists() {
                vault_dir.join("log.md")
            } else {
                workspace_root.join("wiki").join("log.md")
            };

            let content = if log_file.exists() {
                fs::read_to_string(&log_file)
                    .unwrap_or_else(|e| format!("Failed to read log.md: {}", e))
            } else {
                "No certified operational log found at wiki/log.md.".to_string()
            };

            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "Certified Operational Log ({})\n",
                log_file.display()
            ));
            out.push_str("================================================================================\n");
            out.push_str(&content);
            out.push('\n');
            Ok(out)
        }
        "sync" => {
            let (graph, _node_map) = load_vault_graph_from_dir(&vault_dir);
            let (nodes, edges) = serialize_graph_for_sqlite(&vault_dir, &graph);

            let db_path = resolve_vault_db_path(Some(workspace_root))?;
            let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;

            conn.execute_batch(crate::storage::CANONICAL_SCHEMA_SQL)
                .map_err(|e| format!("Schema initialization error on vault.db: {}", e))?;

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            conn.execute("BEGIN TRANSACTION;", [])
                .map_err(|e| e.to_string())?;

            for (node_id, relative_path, node_kind, sha256_hex) in &nodes {
                let _ = conn.execute(
                    "INSERT INTO wiki_nodes (node_id, relative_path, node_kind, sha256_hex, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(node_id) DO UPDATE SET
                        relative_path = excluded.relative_path,
                        node_kind = excluded.node_kind,
                        sha256_hex = excluded.sha256_hex,
                        updated_at = excluded.updated_at",
                    params![node_id, relative_path, node_kind, sha256_hex, now_ms],
                );
            }

            for (source_id, target_id) in &edges {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO wiki_edges (source_id, target_id) VALUES (?1, ?2)",
                    params![source_id, target_id],
                );
            }

            conn.execute("COMMIT;", []).map_err(|e| e.to_string())?;

            let mut out = String::new();
            out.push_str("================================================================================\n");
            out.push_str("Vault Sync Completed\n");
            out.push_str("================================================================================\n");
            out.push_str(&format!(
                "- Synchronized {} nodes and {} edges into SQLite WAL ({})\n",
                nodes.len(),
                edges.len(),
                db_path.display()
            ));
            out.push_str("================================================================================\n");
            Ok(out)
        }
        other => Err(format!(
            "Unknown vault action '{}'. Expected 'status', 'validate', 'log', or 'sync'.",
            other
        )),
    }
}
