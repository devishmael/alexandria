---
id: sandbox-runner
title: "Contrato de Subsistema: Sandbox Runner de Vitest (Node --permission), Normalización UNC y Máquina TDD"
type: module
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - module
  - sandbox
  - vitest
  - node-permission
  - tdd
  - gate-3
---

# Contrato de Subsistema: Sandbox Runner de Vitest (Node --permission), Normalización UNC y Máquina TDD

> **Documento Canónico de Módulo:** `alexandria/modules/sandbox-runner.md`  
> **Archivo de Destino en Kernel:** `src/runner/vitest_sandbox.rs`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-005-node-permission-sandboxing]]

---

## 1. Arquitectura de Confinamiento Nativo (Gate 3)

El subsistema **Sandbox Runner** ejecuta las suites de pruebas de `tests/` fuera de la ventana de contexto del modelo, aplicando una política de **Confianza Cero a Nivel de Sistema Operativo** ([[ADR-005-node-permission-sandboxing]]). En lugar de utilizar wrappers de shell vulnerables (`npx vitest`, `npm test` o `node_modules/.bin/vitest.cmd`) que habilitan inyección de comandos o pierden las banderas de seguridad al hacer *fork*, el kernel de Rust invoca **directamente el binario `node`** sobre el punto de entrada ESM `./node_modules/vitest/vitest.mjs`.

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                   SUBPROCESO CONFINADO: tokio::process::Command::new(node)               │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  1. PURGA DE ENTORNO (cmd.env_clear()):                                                  │
│     • Cero filtración de credenciales (.env, AWS_*, GITHUB_TOKEN, DATABASE_URL real)     │
│     • Preservación de `SystemRoot` en Windows (requerido por libuv / Winsock2 DLLs)      │
│     • Redirección unificada de `TMP`, `TEMP` y `TMPDIR` hacia `canon_temp_dir`           │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  2. POLÍTICA DE PERMISOS DEL KERNEL NODE.JS (--permission):                              │
│     • --allow-worker & --allow-addons: Habilita worker_threads de Vitest y addons nativos│
│     • --allow-fs-read=<workspace>,<node_dir>: Lectura acotada al proyecto y runtime Node │
│     • --allow-fs-write=<coverage_dir>,<temp_dir>: Escritura prohibida en src/ y raíz     │
│     • Sin --allow-child-process y sin red externa: Bloquea exfiltración y shells         │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│  3. CANAL DE REPORTE PURO (--outputFile):                                                │
│     • --reporter=json --outputFile=./coverage/alexandria-vitest-report.json              │
│     • Inmune a contaminación de stdout por console.log(), warnings o trazas de usuario   │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Mitigaciones Críticas de Sistema Operativo (Windows & POSIX)

### 2.1. Limpieza de Prefijos Extendidos UNC (`\\?\`) tras `std::fs::canonicalize()`
En Windows, `std::fs::canonicalize()` devuelve rutas en formato de longitud extendida Win32 (`\\?\C:\Github\...` o `\\?\UNC\server\share\...`). El motor interno de permisos de Node.js (`--allow-fs-read`) y el resolutor ESM de Vite/Rollup fallan con `ERR_ACCESS_DENIED` o `ERR_INVALID_FILE_URL_PATH` cuando reciben el prefijo `\\?\`. La función `clean_canonical_path` normaliza deterministamente estas rutas antes de construir los argumentos del proceso.

### 2.2. Prevención de `ERR_ACCESS_DENIED` en Directorios Temporales
Vitest y Vite compilan archivos de configuración al vuelo (`vitest.config.ts.timestamp-*.mjs`) y cachés intermedias en el directorio temporal del sistema. Si `TMP`/`TEMP` apuntan a una ruta con enlaces simbólicos no resueltos o fuera de `--allow-fs-write`, el proceso aborta inmediatamente. El runner canonicaliza `std::env::temp_dir()`, lo inyecta en `TMP`, `TEMP` y `TMPDIR`, y lo añade explícitamente a `--allow-fs-write`.

---

## 3. Máquina de Estados Semántica para Validación TDD

El modelo en espacio de usuario (`[CONSTRUCTOR]`) no tiene permitido editar `src/` sin haber demostrado primero una **Fase RED legítima**. El evaluador `evaluate_tdd_red_phase` inspecciona la estructura deserializada `VitestJsonReport`:

```
                   [ Lectura de ./coverage/alexandria-vitest-report.json ]
                                              │
             ┌────────────────────────────────┼────────────────────────────────┐
             ▼                                ▼                                ▼
  [ Reporte No Existe /            [ num_failed_tests == 0 ]        [ num_failed_tests > 0 ]
    num_total_suites == 0 /                   │                                │
    assertion_results.is_empty() ]            ▼                                ▼
             │                     ┌─────────────────────┐      ¿Mensaje contiene SyntaxError,
             │                     │   UnexpectedPass    │       ReferenceError o Cannot find?
             │                     │ (RECHAZADO EN RED)  │            ├──► SÍ ──┐
             ▼                     └─────────────────────┘            │         │
┌────────────────────────────────────────────────────────┐            │         ▼
│             InvalidSyntaxOrImportError                 │◄───────────┘  (RECHAZADO EN RED)
│ (Fallo estructural; NO cuenta como prueba de negocio)  │
└────────────────────────────────────────────────────────┘
                                                                      ├──► NO ──┐
                                                                                ▼
                                                               ┌─────────────────────────────────┐
                                                               │      ValidAssertionFailure      │
                                                               │ (FASE RED APROBADA -> Ir GREEN) │
                                                               └─────────────────────────────────┘
```

---

## 4. Implementación Completa y Ejecutable en Rust (`src/runner/vitest_sandbox.rs`)

```rust
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

/// Elimina de forma pura los prefijos extendidos Win32 (`\\?\UNC\` y `\\?\`)
pub fn strip_windows_unc_prefix(raw_path: &str) -> PathBuf {
    if let Some(unc_rest) = raw_path.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", unc_rest));
    }
    if let Some(stripped) = raw_path.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    PathBuf::from(raw_path)
}

/// Limpia el prefijo UNC `\\?\` en Windows para compatibilidad con Node.js `--permission`
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
```

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Runtime Central:** [[core-engine]]
- **Servidor MCP:** [[mcp-server]]
- **Escáner AST Gate 0:** [[gate-guard-ast]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-005-node-permission-sandboxing]]
- **Registro Forense:** [[failures]]
