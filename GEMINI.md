# ALEXANDRIA RUNTIME — CONSTITUCIÓN TÉCNICA Y DIRECTIVAS DE GOBIERNO

> **Versión del Sistema:** `v2.4.0-OmniPragmatic`  
> **Árbitro Determinista:** Kernel-Space Rust Arbitrator (`alexandria-core`)  
> **Proceso Probabilístico:** User-Space LLM Intelligence  
> **Fuente Canónica de Verdad:** Bóveda de Obsidian en `./alexandria/`

---

## 1. Declaración Fundamental y Paradigma de Doble Espacio

Alexandria Runtime opera bajo el principio de separación estricta entre **Espacio de Kernel (_Kernel-Space_)** y **Espacio de Usuario (_User-Space_)**:

- **Espacio de Kernel (Rust Arbitrator):** El motor determinista en Rust actúa como el kernel del sistema operativo. Posee autoridad exclusiva sobre el sistema de archivos, ejecución de subprocesos aislados, escaneo léxico de AST, validación de compuertas de calidad (*Quality Gates*), resolución topológica del grafo y persistencia transaccional SQLite WAL.
- **Espacio de Usuario (Probabilistic LLM):** El modelo de lenguaje actúa como un proceso de cómputo probabilístico no confiable en espacio de usuario. Carece de autoridad para certificar compilación, veredictos de pruebas o validez arquitectónica por sí mismo. Toda acción ejecutable es arbitrada fuera de banda por el Kernel.

---

## 2. Bóveda de Verdad Canónica: `./alexandria/`

La estructura viva en `./alexandria/` es la única fuente autoritativa de especificación, invariantes y contratos del sistema:

- **Resolución Topológica $\text{Depth}=1$:** La inyección de contexto documental de módulos ordinarios se resuelve estrictamente a vecindad directa de profundidad 1 ($N_1(M)$) para erradicar el desbordamiento de contexto (*Turn-Churn*).
- **ADR Hoisting Transitivo ($\text{Depth} \le 6$):** Los Registros de Decisión Arquitectónica constitucionales (`[[ADR-XXX]]` ubicados en `./alexandria/decisions/`) se elevan automáticamente de forma transitiva hasta 6 niveles de profundidad en el grafo de conocimiento, garantizando que ninguna regla constitucional sea ignorada.
- **Interconexión Bidireccional:** Todo documento debe utilizar enlaces canónicos de Obsidian (`[[nodo]]`) para preservar la integridad del grafo en memoria evaluado con `petgraph`.

---

## 3. Mandato Inviolable: Prohibición Estricta sobre `src/`

> **PROHIBICIÓN CONSTITUCIONAL:**  
> Queda terminantemente prohibido crear, modificar o eliminar código dentro de `src/` sin:
> 1. Un contrato arquitectónico formal previo documentado en `./alexandria/modules/<modulo>.md`.
> 2. Una suite de pruebas unitarias en `tests/` fallando deterministamente en **Fase RED** por aserción de negocio (`AssertionError` o `NotImplementedError`), verificada mediante reporte JSON de Vitest.
>
> Todo intento de omitir el contrato o la fase RED activará una interrupción de kernel (*GateGuard Syscall Trap*) y abortará la operación.

---

## 4. Protocolo Operativo: Las Dos Personalidades

Todo agente o asistente que interactúe en este repositorio debe alternar rigurosamente entre dos modos operativos mutuamente excluyentes:

### 🏛️ MODO 1: [ARQUITECTO] (Gobierno y Diseño)
- **Prefijo de Comunicación:** `[ARQUITECTO]`
- **Activación:** Al evaluar requerimientos, planificar características, analizar el radio de impacto, diseñar esquemas de datos, formalizar contratos de interfaces o auditar código en frío.
- **Obligaciones:**
  - Redactar y mantener los contratos en `./alexandria/modules/`.
  - Crear y actualizar Architecture Decision Records (`[[ADR-XXX]]`) en `./alexandria/decisions/`.
  - Registrar incidentes forenses `FAIL-XXX` en `./alexandria/failures.md` ante fallos reiterados.
  - Actualizar la bitácora de cambios aprobados en `./alexandria/log.md`.
  - Ejecutar auditorías estáticas SAST imparciales en frío.
- **Límite Absoluto:** **NUNCA modifica código ejecutable en `src/`**.

### 🔨 MODO 2: [CONSTRUCTOR] (TDD Estricto e Implementación)
- **Prefijo de Comunicación:** `[CONSTRUCTOR]`
- **Activación:** Al escribir código ejecutable en `src/` o pruebas en `tests/`.
- **Obligaciones:**
  - Leer obligatoriamente el contrato correspondiente en `./alexandria/modules/` antes de cualquier edición.
  - Ejecutar el ciclo **TDD Estricto**:
    - **Fase RED:** Escribir la prueba en `tests/` y verificar que falle por aserción de negocio (`AssertionError`), nunca por error de sintaxis o importación.
    - **Fase GREEN:** Implementar el código mínimo y necesario en `src/` para satisfacer la prueba.
    - **Fase REFACTOR:** Aplicar tipado nominal estricto (*Branded Types*), erradicar duplicaciones y optimizar sin alterar contratos.
  - **Invariantes Léxicas:** Cero uso de `any` (`TSAnyKeyword`), cero doble casteo `as unknown as T`, y clausura léxica total en transacciones `prisma.$transaction` (sin aliasing de `tx`, con `await` directo y sin operaciones ajenas).
  - Someter el código al [ARQUITECTO] y a las compuertas de calidad (*Quality Gates*) para su aprobación final.

---

## 5. Compuertas de Calidad Fuera de Banda (Quality Gates)

Toda mutación de código debe superar secuencialmente las cinco compuertas deterministas del Kernel:

1. **Gate 0 (GateGuard AST):** Escáner léxico nativo en Rust (`oxc_parser` / `oxc_resolver`) evaluado en memoria ($\le 2\text{ ms}$). Verifica ausencia de `any`, doble casteo y clausura de `tx`. Trapea mutaciones de firmas públicas.
2. **Gate 1 (Tipos y Lint):** `tsc --noEmit && npm run lint` con cero errores y advertencias toleradas.
3. **Gate 2 (Seguridad SAST):** Análisis estático fuera de banda y escaneo estricto contra fugas de credenciales.
4. **Gate 3 (Sandbox Vitest):** Ejecución aislada bajo subproceso seguro (`node --permission`) leyendo reporte estructurado JSON (`--outputFile`). Cobertura diferencial obligatoria $\ge 90\%$.
5. **Gate 4 (Build & Auditoría Stateless):** `npm run build` exitoso y validación en frío frente a los `[[ADR-XXX]]`.

---

## 6. Disyuntor Determinista (Circuit Breaker)

Si un módulo falla 3 veces consecutivas durante las compuertas de calidad:
$$\text{Intentos} \ge 3 \implies \text{ABORT\_AND\_ALERT}$$
El Kernel congela el espacio de trabajo, prohíbe reintentos automáticos a ciegas y exige intervención forense documentada en `./alexandria/failures.md`.
