use alexandria::cli::{
    CliCommand, HELP_BANNER, VERSION_BANNER, execute_init_scaffolding, parse_cli_args,
};
use serde_json::Value;
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
        "alexandria_e2e_{}_{}_{}",
        prefix,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("Temporary E2E directory creation succeeded");
    dir
}

#[test]
fn test_cli_version_and_help() {
    // 1. In-memory parser verification
    assert_eq!(
        parse_cli_args(vec!["alexandria".to_string(), "--version".to_string()]),
        Ok(CliCommand::Version)
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".to_string(), "-v".to_string()]),
        Ok(CliCommand::Version)
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".to_string(), "--help".to_string()]),
        Ok(CliCommand::Help)
    );
    assert_eq!(
        parse_cli_args(vec!["alexandria".to_string(), "-h".to_string()]),
        Ok(CliCommand::Help)
    );

    // 2. Real E2E execution of compiled binary for --version and -v
    let bin_path = env!("CARGO_BIN_EXE_alexandria");
    for flag in ["--version", "-v"] {
        let out = Command::new(bin_path)
            .arg(flag)
            .output()
            .expect("Failed to invoke alexandria binary");
        assert!(
            out.status.success(),
            "Flag {} must exit with status code 0",
            flag
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(stdout.trim(), VERSION_BANNER);
    }

    // 3. Real E2E execution of compiled binary for --help and -h
    for flag in ["--help", "-h"] {
        let out = Command::new(bin_path)
            .arg(flag)
            .output()
            .expect("Failed to invoke alexandria binary");
        assert!(
            out.status.success(),
            "Flag {} must exit with status code 0",
            flag
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains(VERSION_BANNER));
        assert!(stdout.contains("init [--vault <path>] [--profile <profile>]"));
        assert!(stdout.contains(HELP_BANNER.lines().next().unwrap_or("")));
    }
}

#[test]
fn test_cli_init_scaffolding() {
    let temp_dir = create_unique_temp_dir("init_default");

    let config_path = execute_init_scaffolding(&temp_dir, None, None)
        .expect("Programmatic initialization in temporary directory must succeed");

    assert_eq!(config_path, temp_dir.join(".alexandria.json"));
    assert!(config_path.exists(), ".alexandria.json must exist");

    // Verify wiki/ directory tree
    let wiki_dir = temp_dir.join("wiki");
    assert!(wiki_dir.join("index.md").is_file(), "wiki/index.md must exist");
    assert!(wiki_dir.join("log.md").is_file(), "wiki/log.md must exist");
    assert!(
        wiki_dir.join("failures.md").is_file(),
        "wiki/failures.md must exist"
    );
    assert!(
        wiki_dir.join("architecture").is_dir(),
        "wiki/architecture/ must exist"
    );
    assert!(
        wiki_dir.join("modules").is_dir(),
        "wiki/modules/ must exist"
    );
    assert!(
        wiki_dir.join("decisions").is_dir(),
        "wiki/decisions/ must exist"
    );

    // Verify Pantheon agent profiles (.agents/agents/{bel,enki,utu,nabu}.md)
    let agents_dir = temp_dir.join(".agents").join("agents");
    let bel_md = fs::read_to_string(agents_dir.join("bel.md")).expect("bel.md must exist");
    let enki_md = fs::read_to_string(agents_dir.join("enki.md")).expect("enki.md must exist");
    let utu_md = fs::read_to_string(agents_dir.join("utu.md")).expect("utu.md must exist");
    let nabu_md = fs::read_to_string(agents_dir.join("nabu.md")).expect("nabu.md must exist");

    assert!(bel_md.contains("max_subagents: 5"));
    assert!(enki_md.contains("max_subagents: 3"));
    assert!(utu_md.contains("max_subagents: 1"));
    assert!(nabu_md.contains("max_subagents: 1"));

    // Verify .alexandria.json content
    let raw_json = fs::read_to_string(&config_path).expect("Read .alexandria.json");
    let parsed: Value = serde_json::from_str(&raw_json).expect("Well-formed JSON");
    assert_eq!(parsed["vaultPath"], "./wiki");
    assert_eq!(parsed["profile"], "mvp-fullstack-ts");
    assert_eq!(parsed["engine"]["strictMode"], true);
    assert_eq!(parsed["engine"]["moduleContextDepth"], 1);
    assert_eq!(parsed["engine"]["maxTransitiveAdrHoistingDepth"], 6);
    assert_eq!(parsed["engine"]["circuitBreakerMaxRetries"], 3);
    assert!(
        parsed["engine"]["protectedPathsForOmegaUpgrade"]
            .as_array()
            .is_some_and(|arr| arr.contains(&Value::String("wiki/decisions/**".to_string())))
    );

    // Verify critical exclusions in .gitignore
    let gitignore_content =
        fs::read_to_string(temp_dir.join(".gitignore")).expect(".gitignore must exist");
    assert!(gitignore_content.contains(".env"));
    assert!(gitignore_content.contains("*.db"));
    assert!(gitignore_content.contains(".alexandria/"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_init_custom_vault_path() {
    let temp_dir = create_unique_temp_dir("init_custom_vault");
    let custom_obsidian_path = "C:/ObsidianVaults/AlexandriaCanonical";
    let custom_profile = "staff-kernel-rust";

    // 1. Verify parsing of --vault and --profile flags
    let parsed_cmd = parse_cli_args(vec![
        "alexandria".to_string(),
        "init".to_string(),
        "--vault".to_string(),
        custom_obsidian_path.to_string(),
        "--profile".to_string(),
        custom_profile.to_string(),
    ])
    .expect("Parsing of --vault and --profile flags succeeded");

    assert_eq!(
        parsed_cmd,
        CliCommand::Init {
            vault_path: Some(custom_obsidian_path.to_string()),
            profile: Some(custom_profile.to_string()),
        }
    );

    // 2. E2E execution of binary with `init --vault <path> --profile <profile>`
    let bin_path = env!("CARGO_BIN_EXE_alexandria");
    let output = Command::new(bin_path)
        .current_dir(&temp_dir)
        .args([
            "init",
            "--vault",
            custom_obsidian_path,
            "--profile",
            custom_profile,
        ])
        .output()
        .expect("Failed to execute `alexandria init --vault` in subprocess");

    assert!(
        output.status.success(),
        "Command `alexandria init --vault` must exit with code 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_content = fs::read_to_string(temp_dir.join(".alexandria.json"))
        .expect("Read .alexandria.json generated by CLI");
    let parsed_config: Value =
        serde_json::from_str(&config_content).expect("Valid configuration JSON");

    assert_eq!(
        parsed_config["vaultPath"].as_str(),
        Some(custom_obsidian_path),
        "Flag --vault must configure vaultPath in .alexandria.json"
    );
    assert_eq!(
        parsed_config["profile"].as_str(),
        Some(custom_profile),
        "Flag --profile must configure profile in .alexandria.json"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
