---
id: ADR-003-zero-trust-quality-gates
title: "ADR-003: Compuertas de Calidad de Confianza Cero (Zero-Trust Quality Gates) Fuera de Banda y TDD Semántico"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - quality-gates
  - zero-trust
  - tdd
  - u-tu
---

# ADR-003: Compuertas de Calidad de Confianza Cero (Zero-Trust Quality Gates) Fuera de Banda y TDD Semántico

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

Permitir que el mismo proceso probabilístico (LLM) que escribe el código evalúe su corrección o parsee libremente la salida estándar (`stdout`) de las herramientas de prueba introduce vulnerabilidades críticas de calidad:

1. **Falsas Fases RED en TDD:** Cuando se exige TDD a un agente sin validación semántica del error, el agente suele crear un archivo de prueba con errores de sintaxis (`SyntaxError`) o importaciones inexistentes (`Cannot find module`), reclama que la prueba "ya falló" (Fase RED) y procede a editar `src/` sin haber definido una sola aserción de negocio ejecutable.
2. **Contaminación de `stdout`:** Si una prueba o librería imprime texto arbitrario con `console.log()` en la salida estándar, los parsers basados en consola se corrompen o permiten que el agente falsifique resultados en los logs.
3. **Sesgo de Autocorrección del Autor:** Cuando el constructor (`Enki`) audita su propio `git diff`, justifica atajos arquitectónicos debido al compromiso cognitivo con sus turnos previos.

---

## 3. Decisión

Establecemos una arquitectura de **Cinco Compuertas de Calidad Deterministas Fuera de Banda (*Out-of-Band Zero-Trust Quality Gates*)**:

1. **Secuencia Estricta de Ejecución en el Kernel (`verify_and_gate_changes`):**
   - **Gate 0 (GateGuard AST en RAM):** Escaneo léxico en $\le 2\text{ ms}$ mediante [[gate-guard-ast]] ([[ADR-006-oxc-ast-engine]]). Bloquea instantáneamente `any`, `as unknown as T`, violaciones de clausura en `prisma.$transaction` y alteraciones no autorizadas de firmas públicas.
   - **Gate 1 (Chequeo de Tipos y Linter):** Ejecución de `npx tsc --noEmit && npm run lint` exigiendo código de salida `0`.
   - **Gate 2 (Seguridad SAST y Secretos):** Escaneo del `git diff` contra patrones de claves privadas, tokens y vulnerabilidades de inyección.
   - **Gate 3 (Sandbox de Pruebas y Verificación TDD Semántica):** Ejecución aislada de Vitest bajo [[sandbox-runner]] ([[ADR-005-node-permission-sandboxing]]) leyendo exclusivamente el archivo estructurado `--outputFile=./coverage/alexandria-vitest-report.json`.
   - **Gate 4 (Build de Producción y Auditoría Stateless en Frío de `Utu`):** Ejecución de `npm run build` seguida de una inferencia *stateless* independiente donde `Utu` inspecciona únicamente el `git diff`, los `[[ADR-XXX]]` elevados y los veredictos JSON de las compuertas 0 a 3.
2. **Discriminación Matemática de la Fase RED (`evaluate_tdd_red_phase`):**
   - Una Fase RED **solo** se aprueba (`RedPhaseVerdict::ValidAssertionFailure`) si el archivo JSON existe, contiene al menos una suite compilada (`num_total_test_suites > 0`), tiene aserciones ejecutadas (`!assertion_results.is_empty()`), registra fallos (`num_failed_tests > 0`) y ninguna traza contiene `SyntaxError`, `Cannot find module` o `Cannot find package`.

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Erradicación Total de Fakes en TDD:** Imposibilidad técnica de avanzar a Fase GREEN mediante errores de compilación o importación.
- **Inmunidad al Ruido en Consola:** La lectura desde archivo JSON dedicado garantiza parsing determinista en el $100\%$ de las ejecuciones.
- **Independencia Judicial:** `Utu` actúa como un tribunal constitucional sin acceso a las excusas del proceso constructor.

### Compromisos y Costos Asumidos
- Penalización controlada de latencia en el cierre de tarea (*Path Omega*) por la ejecución secuencial de compilación y juicio *stateless*, mitigada porque **Gate 0** aborta en $< 2\text{ ms}$ cualquier código defectuoso antes de lanzar subprocesos pesados.

### Invariantes Constitucionales
- **Invariante G1:** Ninguna transición `RED -> GREEN` es válida sin un veredicto `ValidAssertionFailure` certificado por el parser JSON de Rust.
- **Invariante G2:** En Fase REFACTOR (*Path Omega*), la cobertura diferencial de los módulos de negocio modificados debe ser $\ge 90\%$.

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Escáner Léxico Gate 0:** [[gate-guard-ast]]
- **Ejecutor Aislado Gate 3:** [[sandbox-runner]]
- **Registro Forense:** [[failures]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-005-node-permission-sandboxing]], [[ADR-006-oxc-ast-engine]]
