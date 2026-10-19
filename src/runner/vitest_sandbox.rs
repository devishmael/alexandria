use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedPhaseVerdict {
    ValidAssertionFailure { failed_assertions: usize },
    InvalidSyntaxOrImportError { details: String },
    UnexpectedPass,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitestJsonReport {
    pub num_total_test_suites: usize,
    pub num_failed_tests: usize,
    pub test_results: Vec<VitestSuiteResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitestSuiteResult {
    pub status: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub assertion_results: Vec<VitestAssertionResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitestAssertionResult {
    pub status: String,
    #[serde(default)]
    pub failure_messages: Vec<String>,
}

pub fn strip_windows_unc_prefix(raw_path: &str) -> PathBuf {
    if let Some(unc_rest) = raw_path.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", unc_rest));
    }
    if let Some(stripped) = raw_path.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    PathBuf::from(raw_path)
}

pub fn clean_canonical_path(path: &Path) -> Result<PathBuf, String> {
    let canon = std::fs::canonicalize(path)
        .map_err(|e| format!("Error canonicalizando ruta {:?}: {}", path, e))?;
    #[cfg(target_os = "windows")]
    {
        return Ok(strip_windows_unc_prefix(&canon.to_string_lossy()));
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(canon)
    }
}

pub async fn run_sandboxed_vitest(
    workspace_root: &Path,
    node_binary_path: &Path,
    node_install_dir: &Path,
    target_test_file: &Path,
    timeout_secs: u64,
) -> Result<(bool, String, Option<VitestJsonReport>), String> {
    let canon_workspace = clean_canonical_path(workspace_root)?;
    let canon_node_dir = clean_canonical_path(node_install_dir)?;
    let canon_temp_dir = clean_canonical_path(&std::env::temp_dir())?;

    let vitest_entry = canon_workspace.join("node_modules/vitest/vitest.mjs");
    let coverage_dir = canon_workspace.join("coverage");
    let _ = std::fs::create_dir_all(&coverage_dir);

    let json_report_path = coverage_dir.join("alexandria-vitest-report.json");
    let _ = std::fs::remove_file(&json_report_path);

    let mut cmd = Command::new(node_binary_path);
    cmd.current_dir(&canon_workspace)
        .env_clear()
        .env("NODE_ENV", "test")
        .env("CI", "true")
        .env("TMP", &canon_temp_dir)
        .env("TEMP", &canon_temp_dir)
        .env("TMPDIR", &canon_temp_dir);

    #[cfg(target_os = "windows")]
    if let Ok(sys_root) = std::env::var("SystemRoot") {
        cmd.env("SystemRoot", sys_root);
    }

    cmd.arg("--permission")
        .arg("--allow-worker")
        .arg("--allow-addons")
        .arg(format!("--allow-fs-read={}", canon_workspace.display()))
        .arg(format!("--allow-fs-read={}", canon_node_dir.display()))
        .arg(format!("--allow-fs-write={}", coverage_dir.display()))
        .arg(format!("--allow-fs-write={}", canon_temp_dir.display()))
        .arg(vitest_entry)
        .arg("run")
        .arg(target_test_file)
        .arg("--reporter=json")
        .arg(format!("--outputFile={}", json_report_path.display()))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| "Timeout excedido ejecutando Vitest en Sandbox".to_string())?
        .map_err(|e| format!("Fallo al lanzar el binario node: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

    let report: Option<VitestJsonReport> = std::fs::read_to_string(&json_report_path)
        .ok()
        .and_then(|json_content| serde_json::from_str(&json_content).ok());

    Ok((
        output.status.success(),
        format!("{}\n{}", stdout_str, stderr_str),
        report,
    ))
}

pub fn evaluate_tdd_red_phase(
    report_opt: Option<VitestJsonReport>,
    raw_logs: &str,
) -> RedPhaseVerdict {
    let Some(report) = report_opt else {
        return RedPhaseVerdict::InvalidSyntaxOrImportError {
            details: format!(
                "Vitest no pudo emitir el archivo JSON de reporte (posible SyntaxError fatal): {}",
                raw_logs
            ),
        };
    };

    if report.num_total_test_suites == 0 {
        return RedPhaseVerdict::InvalidSyntaxOrImportError {
            details: "No se encontró ninguna suite ejecutable.".into(),
        };
    }

    for suite in &report.test_results {
        if suite.assertion_results.is_empty() && suite.status == "failed" {
            return RedPhaseVerdict::InvalidSyntaxOrImportError {
                details: format!("La suite falló al cargar o compilar: {}", suite.message),
            };
        }
    }

    if report.num_failed_tests == 0 {
        return RedPhaseVerdict::UnexpectedPass;
    }

    let mut valid_failures = 0usize;
    for suite in &report.test_results {
        for assertion in &suite.assertion_results {
            if assertion.status == "failed" {
                let msg = assertion.failure_messages.join("\n");
                if msg.contains("SyntaxError")
                    || msg.contains("Cannot find package")
                    || msg.contains("Cannot find module")
                {
                    return RedPhaseVerdict::InvalidSyntaxOrImportError { details: msg };
                }
                valid_failures += 1;
            }
        }
    }

    RedPhaseVerdict::ValidAssertionFailure {
        failed_assertions: valid_failures,
    }
}
