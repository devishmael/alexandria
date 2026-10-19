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
        serde_json::from_str(raw_json).expect("Valid Vitest JSON report");
    let verdict = evaluate_tdd_red_phase(Some(report), "STDOUT clean");

    assert_eq!(
        verdict,
        RedPhaseVerdict::ValidAssertionFailure {
            failed_assertions: 2
        },
        "AssertionError or business logic failures must approve the RED phase"
    );
}

#[test]
fn test_red_phase_reject_syntax_error() {
    // Case A: Suite failed to compile/import before running any assertions (assertionResults is empty)
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
        serde_json::from_str(suite_compile_fail_json).expect("Valid JSON");
    let verdict_a = evaluate_tdd_red_phase(Some(report_a), "");
    match verdict_a {
        RedPhaseVerdict::InvalidSyntaxOrImportError { details } => {
            assert!(
                details.contains("SyntaxError"),
                "Must preserve SyntaxError detail: {}",
                details
            );
        }
        other => panic!(
            "Expected InvalidSyntaxOrImportError for suite without assertions, got: {:?}",
            other
        ),
    }

    // Case B: An assertion failed due to a missing module ('Cannot find module')
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

    let report_b: VitestJsonReport = serde_json::from_str(import_fail_json).expect("Valid JSON");
    let verdict_b = evaluate_tdd_red_phase(Some(report_b), "");
    match verdict_b {
        RedPhaseVerdict::InvalidSyntaxOrImportError { details } => {
            assert!(
                details.contains("Cannot find module"),
                "Must classify 'Cannot find module' as invalid for RED phase: {}",
                details
            );
        }
        other => panic!(
            "Expected InvalidSyntaxOrImportError for broken import, got: {:?}",
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

    let report: VitestJsonReport = serde_json::from_str(passing_json).expect("Valid JSON");
    let verdict = evaluate_tdd_red_phase(Some(report), "");

    assert_eq!(
        verdict,
        RedPhaseVerdict::UnexpectedPass,
        "A passing test suite during RED phase must be rejected with UnexpectedPass"
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
                details.contains("Vitest failed to emit the JSON report file")
                    && details.contains("Unexpected end of file"),
                "Must report the missing JSON file along with raw logs: {}",
                details
            );
        }
        other => panic!(
            "Expected InvalidSyntaxOrImportError when report_opt = None, got: {:?}",
            other
        ),
    }

    // Also verify a report with 0 executable test suites
    let empty_suites_json = r#"{
        "numTotalTestSuites": 0,
        "numFailedTests": 0,
        "testResults": []
    }"#;
    let empty_report: VitestJsonReport =
        serde_json::from_str(empty_suites_json).expect("Valid JSON");
    let verdict_empty = evaluate_tdd_red_phase(Some(empty_report), "");
    assert!(
        matches!(
            verdict_empty,
            RedPhaseVerdict::InvalidSyntaxOrImportError { .. }
        ),
        "A report with 0 test suites must be rejected as InvalidSyntaxOrImportError"
    );
}

#[test]
fn test_clean_canonical_path_normalization() {
    // 1. Verification on the real current working directory
    let cwd = std::env::current_dir().expect("Current working directory accessible");
    let cleaned = clean_canonical_path(&cwd).expect("Canonicalization succeeded");
    let cleaned_str = cleaned.to_string_lossy();

    assert!(
        !cleaned_str.starts_with(r"\\?\"),
        "clean_canonical_path must never return extended Win32 prefix '\\\\?\\': {}",
        cleaned_str
    );
    assert!(
        cleaned.is_absolute(),
        "Cleaned canonical path must remain absolute"
    );

    // 2. Deterministic verification on synthetic paths with Win32 and UNC prefixes
    let synthetic_drive = r"\\?\C:\Github\alexandria-runtime\coverage";
    assert_eq!(
        strip_windows_unc_prefix(synthetic_drive),
        PathBuf::from(r"C:\Github\alexandria-runtime\coverage"),
        "Must strip '\\\\?\\' prefix from local drive paths"
    );

    let synthetic_unc = r"\\?\UNC\server01\shared\alexandria-runtime";
    assert_eq!(
        strip_windows_unc_prefix(synthetic_unc),
        PathBuf::from(r"\\server01\shared\alexandria-runtime"),
        "Must transform '\\\\?\\UNC\\server\\share' into '\\\\server\\share'"
    );
}
