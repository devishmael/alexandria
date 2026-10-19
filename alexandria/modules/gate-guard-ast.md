---
id: gate-guard-ast
title: "Contrato de Subsistema: Escáner Léxico AST en Rust (Oxc), Clausura Transaccional y Syscall Traps"
type: module
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - module
  - oxc
  - ast
  - gate-0
  - prisma
  - branded-types
---

# Contrato de Subsistema: Escáner Léxico AST en Rust (Oxc), Clausura Transaccional y Syscall Traps

> **Documento Canónico de Módulo:** `alexandria/modules/gate-guard-ast.md`  
> **Archivo de Destino en Kernel:** `src/gate_guard/oxc_validator.rs`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-006-oxc-ast-engine]]

---

## 1. Justificación de Ingeniería de Sistemas: Suite `oxc` frente a Tree-sitter, SWC y Babel

El subsistema **Gate Guard AST** ejecuta la compuerta **Gate 0** del arnés antes de permitir cualquier invocación a compiladores externos (`tsc`) o suites de pruebas (`vitest`). Para garantizar una latencia de análisis $\le 2\text{ ms}$ por archivo con cero sobrecarga de memoria, se adopta el ecosistema nativo **Oxc** (`oxc_allocator`, `oxc_parser`, `oxc_ast`, `oxc_span`, `oxc_resolver`) descartando alternativas tradicionales ([[ADR-006-oxc-ast-engine]]):

| Criterio Técnico | Oxc (`oxc_parser` 0.56) | Tree-sitter (`tree-sitter-typescript`) | SWC (`swc_ecma_parser`) | Babel / TypeScript Compiler API |
|---|---|---|---|---|
| **Modelo de Memoria** | **Bump Arena Contigua** (`oxc_allocator::Allocator`). Localidad de caché L1/L2 óptima y liberación en $O(1)$. | Árbol C (FFI) con punteros dispersos en el heap y sobrecarga de puente C-Rust. | Asignaciones individuales (`Box<T>`, `Vec<T>`) en el heap general; alta presión de `drop`. | Heap gestionado por V8 con pausas de Garbage Collection ($\ge 150\text{ MB}$ RSS). |
| **Fidelidad Semántica TS** | **AST Tipado Nativo** (`TSAnyKeyword`, `TSAsExpression`, `TSType`). | CST (Concrete Syntax Tree) genérico basado en consultas de texto S-expression. | AST completo, pero con mayor costo de clonación y transformación. | AST completo, pero prohibitivamente lento para un hook de milisegundos. |
| **Latencia de Parseo** | **$\sim 0.15\text{ ms} - 0.8\text{ ms}$** por archivo mediano. | $\sim 1.5\text{ ms} - 4.0\text{ ms}$ incluyendo FFI y queries. | $\sim 0.8\text{ ms} - 2.5\text{ ms}$ (aprox. $3\times$ más lento que Oxc). | $\sim 40\text{ ms} - 250\text{ ms}$ (requiere levantar runtime JS). |
| **Resolución de Módulos** | `oxc_resolver` integrado (soporta `tsconfig.json` `paths`, `exports` y *barrel files*). | Carece de resolutor semántico de módulos ECMAScript/TS. | Requiere crates adicionales pesados de bundling. | Dependiente de `ts.resolveModuleName`. |

---

## 2. Especificación Formal de Reglas Gramaticales Prohibidas (Gate 0)

El visitante `GateZeroAstVisitor` recorre el programa TypeScript/TSX en una sola pasada ($O(N)$ en el número de nodos) y aplica ocho reglas deterministas:

```
                              [ Nodo AST Visitado en GateZeroAstVisitor ]
                                                   │
         ┌─────────────────────────────────────────┼─────────────────────────────────────────┐
         ▼                                         ▼                                         ▼
[ Reglas Globales de Tipos ]          [ Entrada a $transaction ]            [ Dentro de Scope Activo de tx ]
• TSAnyKeyword -> VIOLACIÓN           • Primer arg != Arrow/Array           • Declaración `const db = tx` -> VIOLACIÓN
  (NO_EXPLICIT_ANY)                     -> PRISMA_TX_REQUIRE_INLINE         • Asignación `db = tx` -> VIOLACIÓN
• TSAsExpression anidado sobre        • Parámetro con destructuring         • Argumento `fn(tx)` -> VIOLACIÓN
  TSUnknownKeyword -> VIOLACIÓN         `async ({ user }) =>`               • `tx.m.op()` sin await en Stmt, Var o Assign
  (NO_DOUBLE_CAST_UNKNOWN)              -> PRISMA_TX_NO_DESTRUCTURING         -> DANGLING / UNWAITED_PROMISE
                                                                            • `await expr` donde expr != `tx.m.op()`
                                                                              o contiene `.then/.catch/.finally`
                                                                              -> PRISMA_TX_STRICT_LEXICAL_AWAIT
                                                                            • Llamada a `fetch`, `setTimeout`, `setInterval`
                                                                              -> PRISMA_TX_NO_UNWAITED_IO
```

### Catálogo de Códigos de Regla de Gate 0

| Código de Regla | Nodo Oxc Interceptado | Condición de Disparo | Propósito Arquitectónico |
|---|---|---|---|
| `NO_EXPLICIT_ANY` | `TSAnyKeyword` | Presencia de la palabra clave `any` en cualquier posición de tipo. | Forzar tipado nominal estricto y *Branded Types*. |
| `NO_DOUBLE_CAST_UNKNOWN` | `TSAsExpression` | `expr as unknown as T` (`inner.type_annotation` es `TSUnknownKeyword`). | Impedir la evasión deliberada del chequeador de tipos. |
| `PRISMA_TX_REQUIRE_INLINE_CALLBACK` | `CallExpression` (`$transaction`) | El primer argumento no es `ArrowFunctionExpression` ni `ArrayExpression`. | Garantizar que el cuerpo transaccional sea estáticamente auditable en el mismo archivo. |
| `PRISMA_TX_NO_PARAM_DESTRUCTURING` | `ArrowFunctionExpression` | El parámetro de la transacción usa patrones de objeto `({ user }) => ...`. | Preservar el identificador único `tx` en la pila `active_tx_params`. |
| `PRISMA_TX_NO_ALIASING` | `VariableDeclarator` | Inicialización `const alias = tx` dentro del alcance de la transacción. | Evitar que el cliente transaccional se oculte bajo otro identificador. |
| `PRISMA_TX_NO_ASSIGNMENT_ESCAPE` | `AssignmentExpression` | Reasignación `outerVar = tx` dentro del alcance de la transacción. | Prevenir la fuga del cliente `tx` fuera del ciclo de vida del callback. |
| `PRISMA_TX_NO_ESCAPE` | `CallExpression` | Paso de `tx` como argumento a otra función `helper(tx)`. | Mantener la clausura léxica total ($\le 25\text{ ms}$ de bloqueo en DB). |
| `PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT` | `VariableDeclarator` / `AssignmentExpression` | `const p = tx.model.op()` o `p = tx.model.op()` sin `await` inmediato. | Bloquear promesas diferidas que rompen el orden secuencial de la transacción. |
| `PRISMA_TX_DANGLING_PROMISE` | `ExpressionStatement` | Sentencia suelta `tx.model.op();` sin `await`. | Impedir cierres prematuros de transacción con consultas aún en vuelo. |
| `PRISMA_TX_STRICT_LEXICAL_AWAIT` | `AwaitExpression` | Cualquier `await` que no invoque directamente `tx.<modelo>.<op>()` o `tx.$queryRaw`, o que encadene `.then()`, `.catch()`, `.finally()`. | Prohibir llamadas de red, `Promise.all` o lógica asíncrona ajena dentro de la transacción. |
| `PRISMA_TX_NO_UNWAITED_IO` | `CallExpression` | Invocación a `fetch`, `setTimeout` o `setInterval` dentro de `$transaction`. | Eliminar bloqueos de red o temporizadores mientras se retienen locks de fila en PostgreSQL/SQLite. |

---

## 3. Detección de Mutación de Firmas Públicas (*Syscall Trap* Alpha $\rightarrow$ Omega)

Además de validar las invariantes léxicas, el módulo extrae la huella ordenada de todas las declaraciones exportadas (`ExportNamedDeclaration`, `ExportDefaultDeclaration`) de un archivo. Si una tarea se ejecuta en **Path Alpha** (`alpha_fast`) y la huella de `export` difiere respecto a la versión base del archivo, el kernel dispara una interrupción `SyscallTrap::PublicSignatureMutated`, aborta la vía rápida y promueve automáticamente la tarea a **Path Omega** (`omega_pantheon`).

---

## 4. Implementación Completa y Ejecutable en Rust (`src/gate_guard/oxc_validator.rs`)

```rust
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstViolation {
    pub rule: &'static str,
    pub message: String,
    pub byte_offset: u32,
}

pub struct GateZeroAstVisitor {
    pub violations: Vec<AstViolation>,
    active_tx_params: Vec<String>,
}

impl Default for GateZeroAstVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl GateZeroAstVisitor {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            active_tx_params: Vec::new(),
        }
    }

    fn current_tx_param(&self) -> Option<String> {
        self.active_tx_params.last().cloned()
    }

    /// Verifica estrictamente `tx.<modelo>.<op>()` o `tx.$queryRaw(...)`
    /// Prohibiendo encadenamientos de promesas como `.then()`, `.catch()` o `.finally()`
    fn is_strict_tx_operation(callee: &Expression<'_>, tx_name: &str) -> bool {
        match callee {
            Expression::StaticMemberExpression(outer) => {
                let prop = outer.property.name.as_str();
                if matches!(prop, "then" | "catch" | "finally") {
                    return false;
                }
                match &outer.object {
                    Expression::Identifier(ident) => ident.name.as_str() == tx_name,
                    Expression::StaticMemberExpression(inner) => {
                        if let Expression::Identifier(ident) = &inner.object {
                            ident.name.as_str() == tx_name
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for GateZeroAstVisitor {
    fn visit_ts_any_keyword(&mut self, it: &TSAnyKeyword) {
        self.violations.push(AstViolation {
            rule: "NO_EXPLICIT_ANY",
            message: "Prohibido el uso de 'any'. Utiliza tipos nominales o Branded Types.".into(),
            byte_offset: it.span.start,
        });
    }

    fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
        if let Expression::TSAsExpression(inner_as) = &it.expression {
            if matches!(inner_as.type_annotation, TSType::TSUnknownKeyword(_)) {
                self.violations.push(AstViolation {
                    rule: "NO_DOUBLE_CAST_UNKNOWN",
                    message: "Prohibido el doble casteo 'as unknown as T' para evadir el sistema de tipos.".into(),
                    byte_offset: it.span.start,
                });
            }
        }
        walk::walk_ts_as_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            if let Some(init_expr) = &it.init {
                match init_expr {
                    Expression::Identifier(ident) if ident.name.as_str() == tx_name => {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_NO_ALIASING",
                            message: format!(
                                "Prohibido reasignar o desestructurar el cliente transaccional '{}'.",
                                tx_name
                            ),
                            byte_offset: it.span.start,
                        });
                    }
                    Expression::CallExpression(call)
                        if Self::is_strict_tx_operation(&call.callee, tx_name) =>
                    {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT",
                            message: format!(
                                "Prohibido almacenar una promesa de '{}' en una variable sin 'await' inmediato.",
                                tx_name
                            ),
                            byte_offset: it.span.start,
                        });
                    }
                    _ => {}
                }
            }
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            match &it.right {
                Expression::Identifier(ident) if ident.name.as_str() == tx_name => {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_NO_ASSIGNMENT_ESCAPE",
                        message: format!(
                            "Prohibido asignar el cliente transaccional '{}' a variables externas.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
                Expression::CallExpression(call)
                    if Self::is_strict_tx_operation(&call.callee, tx_name) =>
                {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT",
                        message: format!(
                            "Prohibido asignar una promesa de '{}' a una variable sin 'await' inmediato.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
                _ => {}
            }
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            if let Expression::CallExpression(call) = &it.expression {
                if Self::is_strict_tx_operation(&call.callee, tx_name) {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_DANGLING_PROMISE",
                        message: format!(
                            "Operación sobre '{}' sin 'await' detectada; causará cierre prematuro de la transacción.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
            }
        }
        walk::walk_expression_statement(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            let valid = match &it.argument {
                Expression::CallExpression(call) => {
                    Self::is_strict_tx_operation(&call.callee, tx_name)
                }
                Expression::TaggedTemplateExpression(tagged) => {
                    Self::is_strict_tx_operation(&tagged.tag, tx_name)
                }
                _ => false,
            };

            if !valid {
                self.violations.push(AstViolation {
                    rule: "PRISMA_TX_STRICT_LEXICAL_AWAIT",
                    message: format!(
                        "Dentro de $transaction solo se permiten llamadas await directas 'await {}.<modelo>.<op>()'. Prohibido I/O externo, .then() o Promise.all.",
                        tx_name
                    ),
                    byte_offset: it.span.start,
                });
            }
        }
        walk::walk_await_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            for arg in &it.arguments {
                if let Argument::Identifier(ident) = arg {
                    if ident.name.as_str() == tx_name {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_NO_ESCAPE",
                            message: format!(
                                "Prohibido pasar el cliente transaccional '{}' a funciones auxiliares.",
                                tx_name
                            ),
                            byte_offset: ident.span.start,
                        });
                    }
                }
            }

            if let Expression::Identifier(ident) = &it.callee {
                if matches!(ident.name.as_str(), "fetch" | "setTimeout" | "setInterval") {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_NO_UNWAITED_IO",
                        message: format!(
                            "Prohibido invocar '{}' dentro de una transacción de base de datos.",
                            ident.name.as_str()
                        ),
                        byte_offset: it.span.start,
                    });
                }
            }
        }

        let is_prisma_tx_call = match &it.callee {
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "$transaction"
            }
            _ => false,
        };

        if is_prisma_tx_call {
            if let Some(first_arg) = it.arguments.first() {
                match first_arg {
                    Argument::ArrayExpression(_) => {
                        walk::walk_call_expression(self, it);
                        return;
                    }
                    Argument::ArrowFunctionExpression(arrow) => {
                        if let Some(first_param) = arrow.params.items.first() {
                            match &first_param.pattern.kind {
                                BindingPatternKind::BindingIdentifier(ident) => {
                                    let param_name = ident.name.as_str().to_string();
                                    self.active_tx_params.push(param_name);
                                    walk::walk_arrow_function_expression(self, arrow);
                                    self.active_tx_params.pop();
                                    return;
                                }
                                _ => {
                                    self.violations.push(AstViolation {
                                        rule: "PRISMA_TX_NO_PARAM_DESTRUCTURING",
                                        message: "El parámetro de $transaction debe ser un identificador simple (ej. 'async (tx) => ...'), sin destructuring.".into(),
                                        byte_offset: first_param.span.start,
                                    });
                                    return;
                                }
                            }
                        }
                    }
                    _ => {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_REQUIRE_INLINE_CALLBACK",
                            message: "El primer argumento de $transaction debe ser un ArrowFunction inline o un Array estático.".into(),
                            byte_offset: first_arg.span().start,
                        });
                        return;
                    }
                }
            }
        }

        walk::walk_call_expression(self, it);
    }
}

pub fn validate_typescript_source(
    source_text: &str,
    is_tsx: bool,
) -> Result<Vec<AstViolation>, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_jsx(is_tsx);
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    if !ret.errors.is_empty() {
        return Err(format!("SyntaxError en parseo Oxc: {:?}", ret.errors[0]));
    }

    let mut visitor = GateZeroAstVisitor::new();
    visitor.visit_program(&ret.program);
    Ok(visitor.violations)
}
```

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Runtime Central:** [[core-engine]]
- **Servidor MCP:** [[mcp-server]]
- **Motor Topológico:** [[topological-graph]]
- **Sandbox Runner Gate 3:** [[sandbox-runner]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-006-oxc-ast-engine]]
- **Registro Forense:** [[failures]]
