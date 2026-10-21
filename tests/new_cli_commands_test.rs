use alexandria::cli::{
    CliCommand, parse_cli_args, execute_init_scaffolding,
    execute_doctor_diagnostic, execute_gate_check, execute_capsule_command,
    execute_harness_scaffold, execute_vault_command,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn create_unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "alexandria_dx_{}_{}_{}",
        prefix,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("Temporary DX directory creation succeeded");
    dir
}

#[test]
fn test_cli_parsing_for_new_dx_commands() {
    // 1. Doctor / Status parsing
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "doctor".into()]),
        Ok(CliCommand::Doctor)
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "status".into()]),
        Ok(CliCommand::Doctor)
    );

    // 2. Gate / Check parsing
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "gate".into(), "src/main.ts".into()]),
        Ok(CliCommand::Gate { target_path: "src/main.ts".into() })
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "check".into(), "src/core".into()]),
        Ok(CliCommand::Gate { target_path: "src/core".into() })
    );

    // 3. Capsule command parsing
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "capsule".into(), "list".into()]),
        Ok(CliCommand::Capsule {
            subaction: "list".into(),
            argument: None,
            domain: None,
        })
    );
    assert_eq!(
        parse_cli_args(vec![
            "alexandria".into(), "capsule".into(), "list".into(),
            "--domain".into(), "database".into()
        ]),
        Ok(CliCommand::Capsule {
            subaction: "list".into(),
            argument: None,
            domain: Some("database".into()),
        })
    );
    assert_eq!(
        parse_cli_args(vec![
            "alexandria".into(), "capsule".into(), "search".into(), "postgresql".into()
        ]),
        Ok(CliCommand::Capsule {
            subaction: "search".into(),
            argument: Some("postgresql".into()),
            domain: None,
        })
    );
    assert_eq!(
        parse_cli_args(vec![
            "alexandria".into(), "capsule".into(), "show".into(), "database:postgresql".into()
        ]),
        Ok(CliCommand::Capsule {
            subaction: "show".into(),
            argument: Some("database:postgresql".into()),
            domain: None,
        })
    );

    // 4. Harness add parsing
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "harness".into(), "add".into(), "cursor".into()]),
        Ok(CliCommand::Harness { target: "cursor".into() })
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "harness".into(), "add".into(), "all".into()]),
        Ok(CliCommand::Harness { target: "all".into() })
    );

    // 5. Vault command parsing
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "vault".into(), "status".into()]),
        Ok(CliCommand::Vault { action: "status".into() })
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "vault".into(), "validate".into()]),
        Ok(CliCommand::Vault { action: "validate".into() })
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "vault".into(), "log".into()]),
        Ok(CliCommand::Vault { action: "log".into() })
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".into(), "vault".into(), "sync".into()]),
        Ok(CliCommand::Vault { action: "sync".into() })
    );
}

#[test]
fn test_doctor_diagnostic_execution() {
    let temp_dir = create_unique_temp_dir("doctor_test");
    // Initialize workspace
    execute_init_scaffolding(&temp_dir, None, None).expect("Init workspace must succeed");

    let report = execute_doctor_diagnostic(&temp_dir).expect("Doctor diagnostic must succeed");
    assert!(report.contains("Alexandria System Diagnostic"));
    assert!(report.contains("[✓] Binary & PATH Validation"));
    assert!(report.contains("[✓] SQLite WAL Health Check"));
    assert!(report.contains("[✓] Project Scoping & Vault Graph"));
    assert!(report.contains("[✓] Multi-Harness Configuration"));
    assert!(report.contains("Organism Telemetry"));
}

#[test]
fn test_gate_check_on_clean_and_violating_files() {
    let temp_dir = create_unique_temp_dir("gate_test");

    // Clean TypeScript file
    let clean_file = temp_dir.join("clean.ts");
    fs::write(
        &clean_file,
        "export function add(a: number, b: number): number {\n    return a + b;\n}\n",
    )
    .unwrap();

    let clean_res = execute_gate_check(&clean_file);
    assert!(clean_res.is_ok(), "Clean file must pass Gate checks");
    let (passed, clean_report) = clean_res.unwrap();
    assert!(passed, "Passed status must be true for clean file");
    assert!(clean_report.contains("PASSED"));

    // Violating file with Gate 0 (`any`) and Gate 2 (`eval` & secret)
    let dirty_file = temp_dir.join("dirty.ts");
    fs::write(
        &dirty_file,
        "export const token = \"sk-live-12345678901234567890\";\nexport function runBad(cmd: any): void {\n    eval(cmd);\n}\n",
    )
    .unwrap();

    let dirty_res = execute_gate_check(&dirty_file);
    assert!(dirty_res.is_ok());
    let (passed, dirty_report) = dirty_res.unwrap();
    assert!(!passed, "Violating file must fail Gate checks");
    assert!(dirty_report.contains("NO_EXPLICIT_ANY") || dirty_report.contains("Explicit 'any'"));
    assert!(dirty_report.contains("GATE2_HARDCODED_SECRET") || dirty_report.contains("sk-"));
    assert!(dirty_report.contains("eval("));
}

#[test]
fn test_harness_add_scaffolding() {
    let temp_dir = create_unique_temp_dir("harness_test");

    // Test scaffolding 'all'
    let summary = execute_harness_scaffold(&temp_dir, "all")
        .expect("Scaffolding all harnesses must succeed");
    assert!(summary.contains("Scaffolded"));

    // Verify all harness files exist
    assert!(temp_dir.join(".cursor").join("rules").join("alexandria.mdc").exists());
    assert!(temp_dir.join(".cursorrules").exists());
    assert!(temp_dir.join(".mcp.json").exists());
    assert!(temp_dir.join("GEMINI.md").exists());
    assert!(temp_dir.join("AGENTS.md").exists());
    assert!(temp_dir.join(".claude").join("CLAUDE.md").exists());
    assert!(temp_dir.join(".codex").join("config.toml").exists());
}

#[test]
fn test_init_automatically_invokes_harness_add_all() {
    let temp_dir = create_unique_temp_dir("init_harness_all");
    execute_init_scaffolding(&temp_dir, None, None).expect("Init must succeed");

    // Verify harness files were auto-created by init
    assert!(temp_dir.join(".cursor").join("rules").join("alexandria.mdc").exists());
    assert!(temp_dir.join("GEMINI.md").exists());
    assert!(temp_dir.join("AGENTS.md").exists());
    assert!(temp_dir.join(".claude").join("CLAUDE.md").exists());
    assert!(temp_dir.join(".mcp.json").exists());
}

#[test]
fn test_capsule_subcommands() {
    let temp_dir = create_unique_temp_dir("capsule_test");
    execute_init_scaffolding(&temp_dir, None, None).expect("Init must succeed");

    // 1. List capsules
    let list_out = execute_capsule_command(&temp_dir, "list", None, None)
        .expect("Capsule list must succeed");
    assert!(list_out.contains("Curated Capsules"));

    // 2. List with domain filter
    let domain_out = execute_capsule_command(&temp_dir, "list", None, Some("backend"))
        .expect("Capsule list by domain must succeed");
    assert!(domain_out.contains("backend") || domain_out.contains("Capsules"));

    // 3. Search capsules
    let search_out = execute_capsule_command(&temp_dir, "search", Some("postgresql"), None)
        .expect("Capsule search must succeed");
    assert!(search_out.contains("Search Results") || search_out.contains("postgresql"));

    // 4. Show capsule
    let show_out = execute_capsule_command(&temp_dir, "show", Some("backend:rust-engineer"), None)
        .expect("Capsule show must succeed");
    assert!(show_out.contains("Capsule:") || show_out.contains("backend:rust-engineer") || show_out.contains("Overview"));
}

#[test]
fn test_vault_subcommands() {
    let temp_dir = create_unique_temp_dir("vault_test");
    execute_init_scaffolding(&temp_dir, None, None).expect("Init must succeed");

    // 1. Vault status
    let status_out = execute_vault_command(&temp_dir, "status").expect("Vault status must succeed");
    assert!(status_out.contains("Knowledge Vault Status"));
    assert!(status_out.contains("Nodes:"));

    // 2. Vault validate
    let validate_out = execute_vault_command(&temp_dir, "validate").expect("Vault validate must succeed");
    assert!(validate_out.contains("Vault Validation"));
    assert!(validate_out.contains("Zero broken references") || validate_out.contains("Acyclicity"));

    // 3. Vault log
    let log_out = execute_vault_command(&temp_dir, "log").expect("Vault log must succeed");
    assert!(log_out.contains("Operational Log") || log_out.contains("log.md"));

    // 4. Vault sync
    let sync_out = execute_vault_command(&temp_dir, "sync").expect("Vault sync must succeed");
    assert!(sync_out.contains("Synchronized") || sync_out.contains("Vault Sync"));
}

#[test]
fn test_binary_e2e_dx_commands() {
    let bin_path = env!("CARGO_BIN_EXE_alexandria");
    let temp_dir = create_unique_temp_dir("e2e_dx_test");

    // 1. Initialize
    let init_out = Command::new(bin_path)
        .current_dir(&temp_dir)
        .arg("init")
        .output()
        .expect("alexandria init must execute");
    assert!(init_out.status.success());

    // 2. Doctor
    let doctor_out = Command::new(bin_path)
        .current_dir(&temp_dir)
        .arg("doctor")
        .output()
        .expect("alexandria doctor must execute");
    assert!(doctor_out.status.success());
    let doc_str = String::from_utf8_lossy(&doctor_out.stdout);
    assert!(doc_str.contains("Alexandria System Diagnostic"));

    // 3. Gate on clean file
    let clean_ts = temp_dir.join("index.ts");
    fs::write(&clean_ts, "export const PI = 3.14159;\n").unwrap();
    let gate_clean = Command::new(bin_path)
        .current_dir(&temp_dir)
        .args(["gate", "index.ts"])
        .output()
        .expect("alexandria gate index.ts must execute");
    assert!(gate_clean.status.success());

    // 4. Gate on dirty file must exit with code 1
    let dirty_ts = temp_dir.join("bad.ts");
    fs::write(&dirty_ts, "export let badVal: any = 'test';\n").unwrap();
    let gate_dirty = Command::new(bin_path)
        .current_dir(&temp_dir)
        .args(["gate", "bad.ts"])
        .output()
        .expect("alexandria gate bad.ts must execute");
    assert_eq!(gate_dirty.status.code(), Some(1), "Gate violation must exit with 1");

    // 5. Vault status
    let vault_status = Command::new(bin_path)
        .current_dir(&temp_dir)
        .args(["vault", "status"])
        .output()
        .expect("alexandria vault status must execute");
    assert!(vault_status.status.success());
}
