---
name: quota-limits
description: "Políticas de preservación de cuota y ventanas móviles de 5 horas para Alexandria Runtime."
---
# Políticas de Cuota y Ventanas Móviles

1. **Modo de Diseño y Gobierno (`--mode planning`):**
   - Asignar `--mode planning` exclusivamente para la redacción de `[[ADR-XXX]]` en `alexandria/decisions/`, el diseño formal de contratos en `alexandria/modules/` y las auditorías SAST en frío ejecutadas por `[ARQUITECTO]` (`Bel` / `Utu`).
2. **Modo de Ejecución y Construcción (`--mode fast`):**
   - Conmutar obligatoriamente a `--mode fast` durante el picado masivo de código, refactorizaciones nominales y generación/ejecución de suites de pruebas en `[CONSTRUCTOR]` (`Enki`), evitando el agotamiento prematuro de la cuota Pro de 5 horas.
3. **Continuidad Desatendida:**
   - Ante cualquier señal de límite de cuota en el modelo principal, el orquestador mantiene la ejecución activa delegando en subagentes bajo `--mode fast` y sincronizando cada hito en `working.md`.
