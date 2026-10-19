---
id: ADR-006-oxc-ast-engine
title: "ADR-006: Adopción de la Suite Nativa Oxc (oxc_parser y oxc_resolver) para Validación AST en Memoria"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - oxc
  - ast
  - arena-allocator
  - gate-0
---

# ADR-006: Adopción de la Suite Nativa Oxc (`oxc_parser` y `oxc_resolver`) para Validación AST en Memoria

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

El arnés requiere validar sincrónicamente en **Gate 0** (antes de lanzar compiladores o pruebas) que el código TypeScript/TSX producido por el constructor cumpla con leyes estructurales estrictas:
1. Ausencia total de `any` (`TSAnyKeyword`) y del doble casteo `as unknown as T`.
2. Clausura léxica total en transacciones `prisma.$transaction` (sin aliasing `const db = tx`, sin encadenamientos `.then()`, sin fuga de `tx` a helpers y sin promesas huérfanas sin `await`).
3. Resolución del grafo de `import` para verificar la disyunción de archivos antes de clonar *Git worktrees* paralelos.

Evaluamos tres familias tecnológicas para este motor:
- **TypeScript Compiler API / ESLint en Node.js:** Requiere levantar un subproceso V8 pesado ($\sim 100\text{ ms} - 500\text{ ms}$ de arranque) y consume más de $150\text{ MB}$ de RAM.
- **Tree-sitter (`tree-sitter-typescript`):** Basado en gramáticas C GLR con llamadas FFI C-Rust; genera un Árbol de Sintaxis Concreta (CST) genérico donde distinguir semánticamente un `TSAsExpression` anidado de una llamada genérica resulta propenso a errores y más lento por la indirección de punteros.
- **SWC (`swc_ecma_parser`):** Escrito en Rust, pero aloja cada nodo del AST de forma individual en el heap del sistema (`Box<T>`), incurriendo en fragmentación de memoria y sobrecarga al liberar árboles grandes.

---

## 3. Decisión

Adoptamos el ecosistema nativo en Rust **Oxc (The Oxidation Compiler)** —específicamente `oxc_allocator`, `oxc_parser`, `oxc_ast`, `oxc_span` y `oxc_resolver`— como motor exclusivo de análisis estático y resolución de módulos en [[gate-guard-ast]] y [[topological-graph]]:

1. **Parseo sobre Arena Contigua (`oxc_allocator::Allocator`):**
   - Todos los nodos del AST de un archivo se alojan secuencialmente en un *bump allocator* lineal en memoria RAM.
   - El recorrido mediante el trait `oxc_ast::Visit` aprovecha la localidad espacial en las cachés L1/L2 del procesador, completando la validación de **Gate 0** en $\sim 0.15\text{ ms} - 0.8\text{ ms}$ por archivo.
   - Al finalizar la inspección del archivo, toda la arena se libera en tiempo constante $O(1)$ sin invocar destructores nodo por nodo.
2. **Visitante Tipado de Clausura Transaccional (`GateZeroAstVisitor`):**
   - Se implementa una pila de alcance léxico (`active_tx_params: Vec<String>`) dentro del visitante `Visit<'a>` que rastrea el parámetro de `$transaction` y audita exhaustivamente `VariableDeclarator`, `AssignmentExpression`, `ExpressionStatement`, `AwaitExpression` y `CallExpression`.
3. **Resolución Nativa de Módulos con `oxc_resolver`:**
   - Para el cálculo de disyunción de código en `spawn_isolated_worktrees`, `oxc_resolver` resuelve alias de `tsconfig.json` (`paths`), `package.json` (`exports`) e `index.ts` en microsegundos sin invocar a Node.js.

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Latencia de Gate 0 Sub-Milisegundo:** Permite interceptar violaciones de tipos o transacciones defectuosas instantáneamente sin gastar ciclos de CPU en `tsc` o `vitest`.
- **Huella de Memoria Constante:** El *bump allocator* reutilizable mantiene el consumo de RAM del binario por debajo de los $15\text{ MB}$.
- **Seguridad de Memoria 100% Rust:** Cero dependencias de compiladores C/C++ para el análisis sintáctico (a diferencia de Tree-sitter).

### Compromisos y Costos Asumidos
- Las versiones de `oxc_allocator`, `oxc_ast`, `oxc_parser` y `oxc_span` en `Cargo.toml` deben actualizarse siempre en bloque sincronizado (*lockstep versioning*) porque comparten definiciones de tipos con tiempos de vida (`'a`) ligados a la arena.

### Invariantes Constitucionales
- **Invariante X1:** Todo archivo `.ts` o `.tsx` modificado debe aprobar `validate_typescript_source` con `violations.is_empty()` antes de ejecutar cualquier subproceso de Gate 1 a Gate 4.
- **Invariante X2:** Las dependencias de la familia `oxc_*` en [[core-engine]] deben fijarse en el mismo tren de versión menor (`0.56`).

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Implementación del Visitante AST:** [[gate-guard-ast]]
- **Disyunción de Imports:** [[topological-graph]]
- **Manifiesto Cargo:** [[core-engine]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]]
