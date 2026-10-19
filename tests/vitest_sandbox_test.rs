use alexandria::runner::vitest_sandbox::{
    RedPhaseVerdict, VitestJsonReport, clean_canonical_path, evaluate_tdd_red_phase,
    strip_windows_unc_prefix,
};
use std::path::PathBuf;

#[test]
fn test_red_phase_valid_assertion_failure() {
    let raw_json = r#"{
        "numTotalTestSuites": 1,
        "numFailedTests": 2,
        "testResults": [
            {
                "status": "failed",
                "message": "",
                "assertionResults": [
                    {
                        "status": "failed",
                        "failureMessages": [
                            "AssertionError: expected true to be false\n    at Object.<anonymous> (tests/billing.spec.ts:14:22)"
                        ]
                    },
                    {
                        "status": "failed",
                        "failureMessages": [
                            "Error: NotImplementedError: calculateInvoiceTotal is not implemented yet"
                        ]
                    }
                ]
            }
        ]
    }"#;

    let report: VitestJsonReport =
        serde_json::from_str(raw_json).expect("Reporte JSON de Vitest válido");
    let verdict = evaluate_tdd_red_phase(Some(report), "STDOUT clean");

    assert_eq!(
        verdict,
        RedPhaseVerdict::ValidAssertionFailure {
            failed_assertions: 2
        },
        "Las fallas por AssertionError o lógica de negocio deben aprobar la Fase RED"
    );
}

#[test]
fn test_red_phase_reject_syntax_error() {
    // Caso A: La suite falló al compilar/importar antes de ejecutar aserciones (assertionResults vacío)
    let suite_compile_fail_json = r#"{
        "numTotalTestSuites": 1,
        "numFailedTests": 0,
        "testResults": [
            {
                "status": "failed",
                "message": "SyntaxError: Unexpected token '{' in tests/broken.spec.ts",
                "assertionResults": []
            }
        ]
    }"#;

    let report_a: VitestJsonReport =
        serde_json::from_str(suite_compile_fail_json).expect("JSON válido");
    let verdict_a = evaluate_tdd_red_phase(Some(report_a), "");
    match verdict_a {
        RedPhaseVerdict::InvalidSyntaxOrImportError { details } => {
            assert!(
                details.contains("SyntaxError"),
                "Debe preservar el detalle del SyntaxError: {}",
                details
            );
        }
        other => panic!(
            "Se esperaba InvalidSyntaxOrImportError para suite sin aserciones, se obtuvo: {:?}",
            other
        ),
    }

    // Caso B: Una aserción falló por módulo inexistente ('Cannot find module')
    let import_fail_json = r#"{
        "numTotalTestSuites": 1,
        "numFailedTests": 1,
        "testResults": [
            {
                "status": "failed",
                "message": "",
                "assertionResults": [
                    {
                        "status": "failed",
                        "failureMessages": [
                            "Error: Cannot find module '../src/non_existent_module' imported from tests/auth.spec.ts"
                        ]
                    }
                ]
            }
        ]
    }"#;

    let report_b: VitestJsonReport = serde_json::from_str(import_fail_json).expect("JSON válido");
    let verdict_b = evaluate_tdd_red_phase(Some(report_b), "");
    match verdict_b {
        RedPhaseVerdict::InvalidSyntaxOrImportError { details } => {
            assert!(
                details.contains("Cannot find module"),
                "Debe detectar 'Cannot find module' como error inválido para Fase RED: {}",
                details
            );
        }
        other => panic!(
            "Se esperaba InvalidSyntaxOrImportError por import roto, se obtuvo: {:?}",
            other
        ),
    }
}

#[test]
fn test_red_phase_reject_unexpected_pass() {
    let passing_json = r#"{
        "numTotalTestSuites": 1,
        "numFailedTests": 0,
        "testResults": [
            {
                "status": "passed",
                "message": "",
                "assertionResults": [
                    {
                        "status": "passed",
                        "failureMessages": []
                    }
                ]
            }
        ]
    }"#;

    let report: VitestJsonReport = serde_json::from_str(passing_json).expect("JSON válido");
    let verdict = evaluate_tdd_red_phase(Some(report), "");

    assert_eq!(
        verdict,
        RedPhaseVerdict::UnexpectedPass,
        "Una prueba que pasa en Fase RED debe rechazarse con UnexpectedPass"
    );
}

#[test]
fn test_red_phase_handle_missing_or_corrupt_json() {
    let raw_fatal_logs =
        "ERROR: Transform failed with 1 error:\ntests/fatal.spec.ts:1:10: ERROR: Unexpected end of file";
    let verdict_none = evaluate_tdd_red_phase(None, raw_fatal_logs);

    match verdict_none {
        RedPhaseVerdict::InvalidSyntaxOrImportError { details } => {
            assert!(
                details.contains("Vitest no pudo emitir el archivo JSON")
                    && details.contains("Unexpected end of file"),
                "Debe reportar la ausencia del archivo JSON junto con el log crudo: {}",
                details
            );
        }
        other => panic!(
            "Se esperaba InvalidSyntaxOrImportError ante report_opt = None, se obtuvo: {:?}",
            other
        ),
    }

    // También verificamos el caso de reporte con 0 suites ejecutables
    let empty_suites_json = r#"{
        "numTotalTestSuites": 0,
        "numFailedTests": 0,
        "testResults": []
    }"#;
    let empty_report: VitestJsonReport =
        serde_json::from_str(empty_suites_json).expect("JSON válido");
    let verdict_empty = evaluate_tdd_red_phase(Some(empty_report), "");
    assert!(
        matches!(
            verdict_empty,
            RedPhaseVerdict::InvalidSyntaxOrImportError { .. }
        ),
        "Un reporte con 0 suites debe rechazarse como InvalidSyntaxOrImportError"
    );
}

#[test]
fn test_clean_canonical_path_normalization() {
    // 1. Verificación sobre el directorio de trabajo real del sistema de archivos
    let cwd = std::env::current_dir().expect("Directorio actual accesible");
    let cleaned = clean_canonical_path(&cwd).expect("Canonicalización exitosa");
    let cleaned_str = cleaned.to_string_lossy();

    assert!(
        !cleaned_str.starts_with(r"\\?\"),
        "clean_canonical_path jamás debe retornar el prefijo Win32 extendido '\\\\?\\': {}",
        cleaned_str
    );
    assert!(
        cleaned.is_absolute(),
        "La ruta canonicalizada limpia debe seguir siendo absoluta"
    );

    // 2. Verificación determinista sobre rutas sintéticas con prefijos Win32 y UNC
    let synthetic_drive = r"\\?\C:\Github\alexandria-runtime\coverage";
    assert_eq!(
        strip_windows_unc_prefix(synthetic_drive),
        PathBuf::from(r"C:\Github\alexandria-runtime\coverage"),
        "Debe eliminar el prefijo '\\\\?\\' de unidades locales"
    );

    let synthetic_unc = r"\\?\UNC\server01\shared\alexandria-runtime";
    assert_eq!(
        strip_windows_unc_prefix(synthetic_unc),
        PathBuf::from(r"\\server01\shared\alexandria-runtime"),
        "Debe transformar '\\\\?\\UNC\\server\\share' en '\\\\server\\share'"
    );
}
