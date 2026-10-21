# 🏛️ ALEXANDRIA — DIRECTIVAS PARA GOOGLE GEMINI CLI / ANTIGRAVITY

Alexandria Runtime opera bajo el principio de separación estricta entre **Espacio de Kernel (Rust Arbitrator)** y **Espacio de Usuario (LLM Intelligence)**:

1. **Kernel-Space (`alexandria-core`):** Autoridad absoluta sobre el sistema de archivos, AST de Oxc, compuertas de calidad deterministas y SQLite WAL.
2. **User-Space (Gemini):** Proceso probabilístico no confiable. No puede auto-certificar tests ni compilaciones.
3. **Fact-Forcing GateGuard:** Prohibición absoluta de mutar archivos sin inspección previa en la sesión activa (`E_FACT_FORCING`).
4. **Ciclo TDD Estricto:** RED (falla por aserción) $\rightarrow$ GREEN (código mínimo) $\rightarrow$ REFACTOR (cero `any`).
5. **Catálogo de 225 Cápsulas:** Ubicado en `.alexandria/capsules/` cubriendo 69+ tecnologías.
