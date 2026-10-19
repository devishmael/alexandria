---
id: ADR-005-node-permission-sandboxing
title: "ADR-005: Confinamiento de Pruebas mediante el Permission Model Nativo de Node.js y Normalización UNC"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - sandbox
  - node-permission
  - vitest
  - security
---

# ADR-005: Confinamiento de Pruebas mediante el Permission Model Nativo de Node.js y Normalización UNC

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

En **Gate 3**, el arnés debe ejecutar suites de pruebas unitarias escritas por un proceso probabilístico (LLM). Si las pruebas se ejecutan directamente sobre el sistema anfitrión sin confinamiento (`npm test`), un fragmento de código malicioso o defectuoso dentro del test podría:
1. Modificar o sobrescribir archivos de producción en `src/`, alterar contratos en `./alexandria/` o borrar el repositorio.
2. Leer secretos del entorno de usuario (`process.env`) o claves privadas SSH del sistema anfitrión.
3. Lanzar subprocesos de shell arbitrarios (`child_process.exec`) o realizar llamadas de red no autorizadas.

Por otro lado, levantar contenedores Docker efímeros por cada ciclo `RED -> GREEN -> REFACTOR` añade entre $1.5\text{ s}$ y $5.0\text{ s}$ de sobrecarga por ejecución, requiere que el daemon de Docker esté instalado y activo en la máquina del desarrollador, y degrada severamente la fluidez del ciclo TDD interactivo.

---

## 3. Decisión

Adoptamos el **Permission Model Nativo de Node.js (`--permission`) con Aislamiento de Entorno en el Kernel de Rust** (implementado en [[sandbox-runner]]) en lugar de contenedores pesados:

1. **Invocación Directa sin Shell Intermedia:**
   - `tokio::process::Command` lanza directamente el ejecutable `node` apuntando a `./node_modules/vitest/vitest.mjs`, evitando `cmd.exe`, `powershell` o `sh` y garantizando que las banderas de permisos se apliquen al proceso raíz de pruebas.
2. **Restricción Estricta de Capacidades (`--permission`):**
   - **Lectura Acotada:** `--allow-fs-read` restringido exclusivamente al directorio canónico del workspace y al directorio de instalación de Node.js.
   - **Escritura Confinada:** `--allow-fs-write` restringido únicamente a `./coverage` y al directorio temporal del sistema (`canon_temp_dir`). Queda bloqueada por el kernel de Node.js cualquier escritura sobre `src/` o `./alexandria/`.
   - **Workers y Addons:** Se habilitan `--allow-worker` (requerido por el pool de hilos de Vitest) y `--allow-addons` (requerido por binarios nativos como Prisma/Rollup), pero se **deniega** `--allow-child-process`.
3. **Purga de Entorno (`env_clear`) y Compatibilidad Multiplataforma:**
   - Se eliminan todas las variables de entorno heredadas mediante `cmd.env_clear()`.
   - En Windows se preserva explícitamente `SystemRoot` (indispensable para que `libuv` inicialice Winsock2 y el generador de entropía del sistema).
   - Se redirigen `TMP`, `TEMP` y `TMPDIR` a `canon_temp_dir` y se eliminan los prefijos extendidos UNC (`\\?\` y `\\?\UNC\`) generados por `std::fs::canonicalize()` para evitar falsos rechazos `ERR_ACCESS_DENIED`.

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Cero Sobrecarga de Virtualización:** El confinamiento se aplica a nivel de binario C++ de Node.js con $0\text{ ms}$ de penalización de arranque frente a una ejecución normal.
- **Protección Física de `src/` y `./alexandria/`:** Incluso si una prueba intenta ejecutar `fs.writeFileSync('src/index.ts', ...)`, el runtime de Node.js aborta la llamada con `ERR_ACCESS_DENIED`.
- **Portabilidad Total:** Funciona nativamente en Windows, macOS y Linux sin requerir Docker Desktop ni privilegios de root.

### Compromisos y Costos Asumidos
- Requiere Node.js $\ge 20.x$ / $22.x$ con soporte estable para `--permission`.

### Invariantes Constitucionales
- **Invariante N1:** Queda prohibido ejecutar Vitest a través de `npx`, `npm` o scripts `.cmd`/`.sh` en Gate 3.
- **Invariante N2:** Toda ruta pasada a `--allow-fs-read` o `--allow-fs-write` debe ser previamente normalizada mediante `clean_canonical_path`.

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Implementación del Sandbox:** [[sandbox-runner]]
- **Runtime Central:** [[core-engine]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]]
