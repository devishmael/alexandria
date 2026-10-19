---
id: ADR-001-kernel-space-harness
title: "ADR-001: Arquitectura de Arnés en Espacio de Kernel (Rust) vs. Proceso Probabilístico en Espacio de Usuario (LLM)"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - kernel-space
  - user-space
  - rust
  - architecture
---

# ADR-001: Arquitectura de Arnés en Espacio de Kernel (Rust) vs. Proceso Probabilístico en Espacio de Usuario (LLM)

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

Los sistemas convencionales de ingeniería asistida por IA delegan el control del ciclo de vida de desarrollo (lectura de documentación, edición de código, ejecución de shells, verificación de pruebas y confirmación de cambios) al propio Modelo de Lenguaje (LLM) mediante bucles de agentes autónomos en Python o TypeScript. Esta arquitectura presenta cuatro modos de fallo estructurales en entornos de producción:

1. **Autocertificación Falaz (*Reward Hacking*):** Un agente probabilístico con permiso para evaluar su propio trabajo tiende a reportar que las pruebas pasaron, omitir casos de borde o relajar reglas de tipado (`any`) cuando enfrenta errores difíciles de compilación.
2. **Degradación de Ventana por Ruido Operativo (*Context Rot*):** La inyección directa de salidas crudas de compiladores, trazas de depuración extensas y volcados completos de documentación degrada la atención del modelo y multiplica el consumo de tokens.
3. **Bucle de Destrucción Iterativa (*LLM Thrashing*):** Ante un fallo no resuelto, los agentes sin supervisión determinista entran en ciclos de ensayo y error que corrompen módulos adyacentes.
4. **Sobrecarga de Recursos del Arnés:** Los arneses basados en runtimes interpretados o con motores de *embeddings* embebidos en proceso consumen cientos de megabytes de RAM por instancia.

---

## 3. Decisión

Establecemos una arquitectura de **Separación de Privilegios inspirada en Kernels de Sistemas Operativos**:

1. **User-Space (Espacio de Usuario No Confiable):**
   - El LLM opera estrictamente como un motor de inferencia y síntesis en espacio de usuario.
   - Carece de autoridad para certificar compilación, aprobar fases TDD o saltarse contratos.
   - Se organiza lógicamente en el **Cuarteto Central** (`Bel`, `Enki`, `Utu`, `Nabu`), comunicándose con el sistema anfitrión exclusivamente a través de llamadas de sistema tipadas vía JSON-RPC 2.0 ([[mcp-server]]).
2. **Kernel-Space (Árbitro Determinista en Rust):**
   - El binario nativo compilado en Rust Edición 2024 ([[core-engine]]) posee el monopolio sobre el acceso a disco, resolución topológica en RAM ([[topological-graph]]), validación léxica AST ([[gate-guard-ast]]), ejecución de subprocesos en sandbox ([[sandbox-runner]]) y persistencia relacional ([[database-schema]]).
   - El binario se compila con `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` y `strip = true`, manteniendo una huella de memoria residente $\le 15\text{ MB}$.
3. **Interrupciones de Sistema (*Syscall Traps*) y Disyuntor (*Circuit Breaker*):**
   - Si una operación en vía rápida (*Path Alpha*) intenta tocar rutas protegidas o alterar firmas públicas exportadas, el kernel dispara una interrupción y promueve la sesión a *Path Omega*.
   - Si un módulo acumula $3$ fallos consecutivos en las compuertas de calidad, el kernel transiciona el estado a `tripped_abort`, congela el árbol de trabajo y exige intervención arquitectónica en [[failures]].

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Determinismo Absoluto:** Ningún cambio llega al árbol de commits sin haber superado verificaciones binarias fuera del control del modelo.
- **Eficiencia de Recursos:** Tiempo de arranque instantáneo ($< 5\text{ ms}$) y huella de memoria despreciable ($\sim 8\text{ MB} - 15\text{ MB}$ RSS).
- **Auditoría Imparcial:** El juicio de `Utu` se ejecuta en una llamada *stateless* en frío, libre del sesgo de confirmación acumulado por `Enki`.

### Compromisos y Costos Asumidos
- Toda nueva herramienta o capacidad del agente requiere definir su contrato estricto en Rust y exponerlo formalmente en el servidor MCP.

### Invariantes Constitucionales
- **Invariante K1:** Queda terminantemente prohibido modificar archivos en `src/` sin un contrato previo en `alexandria/modules/` y sin una prueba fallando por aserción en `tests/`.
- **Invariante K2:** Todo módulo con `consecutive_failures >= 3` queda bloqueado a nivel de kernel hasta que se registre su resolución forense.

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Módulos Gobernados:** [[core-engine]], [[mcp-server]], [[topological-graph]], [[gate-guard-ast]], [[sandbox-runner]]
- **Decisiones Relacionadas:** [[ADR-002-prefix-pinned-caching]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-005-node-permission-sandboxing]], [[ADR-006-oxc-ast-engine]]
