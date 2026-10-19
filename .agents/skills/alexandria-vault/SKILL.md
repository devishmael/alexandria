---
name: alexandria-vault
description: "Resuelve contratos y ADRs desde la carpeta alexandria/ bajo demanda a profundidad 1."
---
# Procedimiento de Invocación
1. Prohibido leer masivamente la carpeta `alexandria/`.
2. Consulta `alexandria/index.md` para resolver la ruta del módulo solicitado.
3. Inyecta únicamente el archivo `alexandria/modules/<modulo>.md` y los `[[ADR-XXX]]` enlazados directamente.
