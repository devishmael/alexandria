# 🏛️ ALEXANDRIA — INSTRUCCIONES DE GOBIERNO PARA CLAUDE CODE

Bienvenido al repositorio de **Alexandria (v1.0.0)**. Este proyecto opera bajo un modelo de **Doble Espacio Estricto**:
- **Kernel-Space (Rust Arbitrator):** El binario en Rust (`alexandria-core`) arbitra todas las mutaciones, compuertas de calidad y acceso al sistema de archivos.
- **User-Space (LLM):** Proceso probabilístico no confiable que debe someterse a las Quality Gates de Alexandria.

## Invariantes Obligatorias para Claude Code

1. **Fact-Forcing Gate:** Está estrictamente prohibido modificar o crear archivos sin antes inspeccionar su contenido o consultar los símbolos afectados.
2. **Ciclo TDD Estricto:** Toda nueva funcionalidad o corrección en `src/` exige:
   - Una prueba en `tests/` fallando deterministamente por aserción en **Fase RED**.
   - Código mínimo para pasar a **Fase GREEN**.
   - **Fase REFACTOR** con tipado nominal y cero `any`.
3. **Catálogo de 225 Cápsulas:** Consulta las cápsulas en `.alexandria/capsules/` para respetar las mejores prácticas e invariantes de más de 69 tecnologías.
4. **Comandos de Verificación:**
   - Pruebas completas: `cargo test --all-targets`
   - Verificación de tipos: `cargo check --all-targets`
   - Servidor MCP: `cargo run --bin alexandria -- mcp`
