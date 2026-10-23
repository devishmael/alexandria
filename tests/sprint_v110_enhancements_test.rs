use alexandria::cli::{
    execute_doctor_diagnostic, execute_harness_scaffold, parse_cli_args, CliCommand,
};
use alexandria::gate_guard::oxc_validator::{
    validate_typescript_source, validate_typescript_source_scoped,
};
use alexandria::vector_store::hybrid_memory::{
    compute_deterministic_int8_embedding, cosine_similarity_int8, FEATURE_VECTOR_DIM,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn create_temp_v110_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "alexandria_v110_{}_{}_{}",
        prefix,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("Temporary directory creation failed");
    dir
}

#[test]
fn test_vector_1_git_pre_commit_hook_scaffolding() {
    let temp_dir = create_temp_v110_dir("git_hook");
    let git_dir = temp_dir.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    // 1. Parsing alexandria harness add git
    assert_eq!(
        parse_cli_args(vec![
            "alexandria".into(),
            "harness".into(),
            "add".into(),
            "git".into()
        ]),
        Ok(CliCommand::Harness {
            target: "git".into()
        })
    );

    // 2. Scaffolding git hook
    let summary = execute_harness_scaffold(&temp_dir, "git")
        .expect("Scaffolding git harness hook must succeed");
    assert!(summary.contains("Git Pre-Commit Hook") || summary.contains("pre-commit"));

    let hook_sh = git_dir.join("hooks").join("pre-commit");
    assert!(hook_sh.exists(), ".git/hooks/pre-commit must exist");
    let hook_content = fs::read_to_string(&hook_sh).unwrap();
    assert!(
        hook_content.contains("alexandria gate"),
        "Hook must execute alexandria gate"
    );

    #[cfg(target_os = "windows")]
    {
        let hook_bat = git_dir.join("hooks").join("pre-commit.bat");
        assert!(hook_bat.exists(), ".git/hooks/pre-commit.bat must exist on Windows");
    }

    // 3. alexandria gate check alias support
    let test_file = temp_dir.join("sample.ts");
    fs::write(&test_file, "export const x: number = 42;\n").unwrap();

    let check_cmd = parse_cli_args(vec![
        "alexandria".into(),
        "gate".into(),
        "check".into(),
        test_file.to_str().unwrap().into(),
    ]);
    assert_eq!(
        check_cmd,
        Ok(CliCommand::Gate {
            target_path: test_file.to_str().unwrap().into()
        })
    );
}

#[test]
fn test_vector_2_128_dimensional_vector_projection() {
    assert_eq!(FEATURE_VECTOR_DIM, 128, "Feature vector must be exactly 128 dimensions");

    let text_a = "Prisma transaction isolation and PostgreSQL MVCC consistency";
    let text_b = "Prisma transaction isolation and PostgreSQL MVCC consistency";
    let text_c = "Frontend React virtual DOM state synchronization";

    let vec_a: [i8; 128] = compute_deterministic_int8_embedding(text_a);
    let vec_b: [i8; 128] = compute_deterministic_int8_embedding(text_b);
    let vec_c: [i8; 128] = compute_deterministic_int8_embedding(text_c);

    assert_eq!(vec_a, vec_b, "Embedding must be 100% deterministic");
    assert_eq!(vec_a.len(), 128);

    let sim_same = cosine_similarity_int8(&vec_a, &vec_b);
    assert!(
        sim_same > 0.99,
        "Identical vectors must have cosine similarity ~1.0, got {}",
        sim_same
    );

    let sim_diff = cosine_similarity_int8(&vec_a, &vec_c);
    assert!(
        sim_diff < 0.60,
        "Unrelated topics must have lower cosine similarity with 128-dim vectors, got {}",
        sim_diff
    );
}

#[test]
fn test_vector_3_doctor_diagnostics_nodejs_and_pnpm() {
    let temp_dir = create_temp_v110_dir("doctor_pnpm");
    let wiki_dir = temp_dir.join("wiki");
    fs::create_dir_all(&wiki_dir).unwrap();
    fs::write(wiki_dir.join("index.md"), "# Index\n").unwrap();

    // Create a mock pnpm-lock.yaml in the project directory
    fs::write(temp_dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();

    let report = execute_doctor_diagnostic(&temp_dir)
        .expect("Doctor diagnostic must run and return report");

    // Must verify Node.js runtime check
    assert!(
        report.contains("Node.js Runtime"),
        "Report must contain Node.js Runtime diagnostic"
    );

    // Must detect package manager and flag pnpm warning
    assert!(
        report.contains("Package Manager: pnpm"),
        "Report must detect pnpm package manager"
    );
    assert!(
        report.contains("symlink") || report.contains("virtual store"),
        "Report must provide advice/warning for pnpm symlink virtual store"
    );
}

#[test]
fn test_vector_4_decoupled_prisma_ast_invariants() {
    // Non-Prisma source code using a generic $transaction function
    let non_prisma_source = r#"
        // Generic database or custom service transaction
        interface CustomDb {
            $transaction(fn: (tx: any) => Promise<void>): Promise<void>;
        }
        async function runCustom(db: CustomDb) {
            await db.$transaction(async (tx) => {
                const helper = tx; // Aliasing tx
                helper.doSomething();
            });
        }
    "#;

    // 1. Without Prisma active: Prisma rules should NOT flag the tx aliasing
    let violations_no_prisma = validate_typescript_source_scoped(non_prisma_source, false, false)
        .expect("Code must parse");
    assert!(
        !violations_no_prisma.iter().any(|v| v.rule.starts_with("PRISMA_TX_")),
        "PRISMA_TX rules must be skipped when Prisma is not in scope"
    );

    // 2. With Prisma explicitly active or imported: Prisma rules DO flag tx aliasing
    let prisma_source = r#"
        import { PrismaClient } from '@prisma/client';
        async function runPrisma(prisma: PrismaClient) {
            await prisma.$transaction(async (tx) => {
                const alias = tx; // Forbidden aliasing in Prisma
                await alias.user.create({ data: { name: "test" } });
            });
        }
    "#;

    let violations_prisma = validate_typescript_source(prisma_source, false)
        .expect("Code must parse");
    assert!(
        violations_prisma.iter().any(|v| v.rule == "PRISMA_TX_NO_ALIASING"),
        "PRISMA_TX_NO_ALIASING must be triggered when Prisma is imported/active"
    );
}
