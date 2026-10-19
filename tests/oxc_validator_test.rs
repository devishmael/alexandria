use alexandria::gate_guard::oxc_validator::validate_typescript_source;

#[test]
fn test_reject_explicit_any() {
    let source = r#"
        export type UserId = string & { readonly __brand: unique symbol };
        let x: any = 5;
        function process(payload: any): void {
            console.log(payload, x);
        }
    "#;

    let violations =
        validate_typescript_source(source, false).expect("El código TS debe parsear sin errores");
    assert!(
        violations.iter().any(|v| v.rule == "NO_EXPLICIT_ANY"),
        "Debe detectar y rechazar TSAnyKeyword"
    );
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.rule == "NO_EXPLICIT_ANY")
            .count(),
        2,
        "Debe detectar ambas ocurrencias explícitas de 'any'"
    );
}

#[test]
fn test_reject_double_cast_unknown() {
    let source = r#"
        interface User {
            id: string;
            email: string;
        }
        const rawInput: Record<string, string> = { id: "123" };
        const forgedUser = rawInput as unknown as User;
    "#;

    let violations =
        validate_typescript_source(source, false).expect("El código TS debe parsear sin errores");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, "NO_DOUBLE_CAST_UNKNOWN");
}

#[test]
fn test_strict_prisma_transaction_valid() {
    let source = r#"
        export type AccountId = string & { readonly __brand: unique symbol };

        async function transferFunds(prisma: PrismaClient, fromId: AccountId, toId: AccountId) {
            await prisma.$transaction(async (tx) => {
                await tx.account.update({
                    where: { id: fromId },
                    data: { balance: { decrement: 100 } },
                });
                await tx.account.update({
                    where: { id: toId },
                    data: { balance: { increment: 100 } },
                });
                await tx.$queryRaw`SELECT 1`;
            });

            await prisma.$transaction([
                prisma.auditLog.create({ data: { action: "TRANSFER" } }),
                prisma.metric.update({ where: { key: "tx_count" }, data: { val: 1 } }),
            ]);
        }
    "#;

    let violations =
        validate_typescript_source(source, false).expect("El código TS válido debe parsear");
    assert!(
        violations.is_empty(),
        "Una transacción limpia con clausura léxica total no debe producir violaciones, pero produjo: {:?}",
        violations
    );
}

#[test]
fn test_reject_tx_promise_chaining() {
    let source = r#"
        async function createWithChain(prisma: PrismaClient) {
            await prisma.$transaction(async (tx) => {
                await tx.user.create({ data: { name: "Enki" } }).then((u) => u.id);
            });
        }
    "#;

    let violations =
        validate_typescript_source(source, false).expect("El código TS debe parsear sin errores");
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "PRISMA_TX_STRICT_LEXICAL_AWAIT"),
        "Debe bloquear encadenamientos .then() sobre promesas dentro de $transaction: {:?}",
        violations
    );
}

#[test]
fn test_reject_tx_aliasing_and_escape() {
    let source = r#"
        function helper(client: PrismaTransactionClient) {
            return client;
        }

        async function badTransactionEscape(prisma: PrismaClient) {
            let externalRef: PrismaTransactionClient | null = null;
            await prisma.$transaction(async (tx) => {
                const db = tx;
                externalRef = tx;
                helper(tx);
                await tx.user.create({ data: { name: "Utu" } });
            });
        }
    "#;

    let violations =
        validate_typescript_source(source, false).expect("El código TS debe parsear sin errores");

    assert!(
        violations.iter().any(|v| v.rule == "PRISMA_TX_NO_ALIASING"),
        "Debe detectar 'const db = tx;': {:?}",
        violations
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "PRISMA_TX_NO_ASSIGNMENT_ESCAPE"),
        "Debe detectar 'externalRef = tx;': {:?}",
        violations
    );
    assert!(
        violations.iter().any(|v| v.rule == "PRISMA_TX_NO_ESCAPE"),
        "Debe detectar 'helper(tx)': {:?}",
        violations
    );
}

#[test]
fn test_reject_unwaited_and_dangling_promises() {
    let source = r#"
        async function danglingOperations(prisma: PrismaClient) {
            await prisma.$transaction(async (tx) => {
                fetch("https://example.com/webhook");
                tx.user.create({ data: { name: "DanglingStmt" } });
                const p = tx.user.create({ data: { name: "AssignedPromise" } });
            });
        }
    "#;

    let violations =
        validate_typescript_source(source, false).expect("El código TS debe parsear sin errores");

    assert!(
        violations
            .iter()
            .any(|v| v.rule == "PRISMA_TX_NO_UNWAITED_IO"),
        "Debe detectar llamada a fetch() dentro de $transaction: {:?}",
        violations
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "PRISMA_TX_DANGLING_PROMISE"),
        "Debe detectar sentencia suelta tx.user.create() sin await: {:?}",
        violations
    );
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT"),
        "Debe detectar 'const p = tx.user.create()' sin await: {:?}",
        violations
    );
}
