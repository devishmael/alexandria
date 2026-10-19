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
    fs::create_dir_all(&dir).expect("Creación de carpeta temporal E2E exitosa");
    dir
}

#[test]
fn test_cli_version_and_help() {
    // 1. Verificación del parser en memoria
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

    // 2. Ejecución E2E real del binario compilado para --version y -v
    let bin_path = env!("CARGO_BIN_EXE_alexandria");
    for flag in ["--version", "-v"] {
        let out = Command::new(bin_path)
            .arg(flag)
            .output()
            .expect("Fallo al invocar binario alexandria");
        assert!(
            out.status.success(),
            "La bandera {} debe finalizar con código de salida 0",
            flag
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(stdout.trim(), VERSION_BANNER);
    }

    // 3. Ejecución E2E real del binario compilado para --help y -h
    for flag in ["--help", "-h"] {
        let out = Command::new(bin_path)
            .arg(flag)
            .output()
            .expect("Fallo al invocar binario alexandria");
        assert!(
            out.status.success(),
            "La bandera {} debe finalizar con código de salida 0",
            flag
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains(VERSION_BANNER));
        assert!(stdout.contains("init [--vault <ruta>] [--profile <perfil>]"));
        assert!(stdout.contains(HELP_BANNER.lines().next().unwrap_or("")));
    }
}

#[test]
fn test_cli_init_scaffolding() {
    let temp_dir = create_unique_temp_dir("init_default");

    let config_path = execute_init_scaffolding(&temp_dir, None, None)
        .expect("Inicialización programática en carpeta temporal debe tener éxito");

    assert_eq!(config_path, temp_dir.join(".alexandria.json"));
    assert!(config_path.exists(), "Debe existir .alexandria.json");

    // Verificación del árbol documental wiki/
    let wiki_dir = temp_dir.join("wiki");
    assert!(wiki_dir.join("index.md").is_file(), "Debe existir wiki/index.md");
    assert!(wiki_dir.join("log.md").is_file(), "Debe existir wiki/log.md");
    assert!(
        wiki_dir.join("failures.md").is_file(),
        "Debe existir wiki/failures.md"
    );
    assert!(
        wiki_dir.join("architecture").is_dir(),
        "Debe existir wiki/architecture/"
    );
    assert!(
        wiki_dir.join("modules").is_dir(),
        "Debe existir wiki/modules/"
    );
    assert!(
        wiki_dir.join("decisions").is_dir(),
        "Debe existir wiki/decisions/"
    );

    // Verificación de validez del contenido de .alexandria.json
    let raw_json = fs::read_to_string(&config_path).expect("Lectura de .alexandria.json");
    let parsed: Value = serde_json::from_str(&raw_json).expect("JSON bien formado");
    assert_eq!(parsed["vaultPath"], "./wiki");
    assert_eq!(parsed["profile"], "mvp-fullstack-ts");
    assert_eq!(parsed["engine"]["strictMode"], true);
    assert_eq!(parsed["engine"]["moduleContextDepth"], 1);
    assert_eq!(parsed["engine"]["maxTransitiveAdrHoistingDepth"], 6);
    assert_eq!(parsed["engine"]["circuitBreakerMaxRetries"], 3);

    // Verificación de exclusiones críticas en .gitignore
    let gitignore_content =
        fs::read_to_string(temp_dir.join(".gitignore")).expect("Debe existir .gitignore");
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

    // 1. Verificación del parser de banderas --vault y --profile
    let parsed_cmd = parse_cli_args(vec![
        "alexandria".to_string(),
        "init".to_string(),
        "--vault".to_string(),
        custom_obsidian_path.to_string(),
        "--profile".to_string(),
        custom_profile.to_string(),
    ])
    .expect("Parseo de banderas --vault y --profile exitoso");

    assert_eq!(
        parsed_cmd,
        CliCommand::Init {
            vault_path: Some(custom_obsidian_path.to_string()),
            profile: Some(custom_profile.to_string()),
        }
    );

    // 2. Ejecución E2E del binario con `init --vault <ruta> --profile <perfil>`
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
        .expect("Fallo al ejecutar `alexandria init --vault` en subproceso");

    assert!(
        output.status.success(),
        "El comando `alexandria init --vault` debe retornar código 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_content = fs::read_to_string(temp_dir.join(".alexandria.json"))
        .expect("Lectura de .alexandria.json generado por CLI");
    let parsed_config: Value =
        serde_json::from_str(&config_content).expect("JSON de configuración válido");

    assert_eq!(
        parsed_config["vaultPath"].as_str(),
        Some(custom_obsidian_path),
        "La bandera --vault debe configurar exactamente vaultPath en .alexandria.json"
    );
    assert_eq!(
        parsed_config["profile"].as_str(),
        Some(custom_profile),
        "La bandera --profile debe configurar exactamente profile en .alexandria.json"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
