---
id: ADR-007-multi-host-adapter-and-persistence
title: "ADR-007: Adaptadores Multi-Host Agnósticos y Proyección de Estado Persistente en Disco (working.md)"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - multi-host
  - antigravity
  - persistence
  - working-md
  - mcp
---

# ADR-007: Adaptadores Multi-Host Agnósticos y Proyección de Estado Persistente en Disco (`working.md`)

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

Los entornos de desarrollo asistidos por IA contemporáneos (Google Antigravity 2.0, Claude Code, Cursor, Windsurf, Codex CLI) presentan mecanismos divergentes y propietarios para el gobierno de agentes, límites de cuota y manejo de contexto:
- Google Antigravity utiliza definiciones de subagentes y habilidades bajo `.agents/agents/`, `.agents/rules/` y `.agents/skills/`.
- Otros clientes dependen de archivos `CLAUDE.md`, `.cursor/rules` o configuraciones `mcp_config.json`.

Además, depender exclusivamente de la memoria volátil de la ventana de conversación del cliente provoca **amnesia operativa** ante:
1. Agotamiento o rotación de ventanas móviles de cuota (ej. ventanas de 5 horas en modelos Pro).
2. Reinicios de sesión del IDE, caídas de red o compactaciones forzadas de historial (`/compact`).
3. Ejecuciones autónomas desatendidas de larga duración (*overnight runs*).

---

## 3. Decisión

Adoptamos una arquitectura de **Desacoplamiento Multi-Host con Proyección de Memoria Física en Disco**, gobernada por el contrato [[host-adapters]]:

1. **Puente de Persistencia Bidireccional (`vault.db` $\leftrightarrow$ `working.md`):**
   - El estado operativo del arnés almacenado en `~/.alexandria/vault.db` ([[database-schema]]) se proyecta deterministamente en el archivo plano `working.md` ubicado en la raíz del repositorio.
   - `working.md` actúa como un *checkpoint* legible por humanos y máquinas que registra el checklist de fases, el último paso verificado, la siguiente acción inmediata y la directiva de respaldo ante agotamiento de cuota.
2. **Capa Formal de Adaptadores (`alexandria::adapters`):**
   - **Adaptador Nativo Google Antigravity (`.agents/`):** Materializa subagentes especializados con concurrencia acotada (`architect.md` con `max_subagents: 5` y modo de diseño vs. `builder.md` con `max_subagents: 3` y modo de ejecución TDD), políticas de preservación de cuota (`.agents/rules/quota-limits.md`) y carga progresiva de contexto (`.agents/skills/alexandria-vault/SKILL.md`).
   - **Adaptador Universal MCP (`mcp_config.json`):** Expone las 6 herramientas del kernel a través de [[mcp-server]] (`./target/release/alexandria mcp`) sobre `stdio`, garantizando paridad funcional en cualquier cliente compatible con JSON-RPC 2.0.
3. **Conmutación Resiliente de Cuota (`--mode planning` vs. `--mode fast`):**
   - Se reserva el razonamiento profundo (`--mode planning`) para la formalización de ADRs, diseño de contratos y auditorías en frío.
   - Se asigna el modo de alta velocidad (`--mode fast`) para la generación iterativa de pruebas unitarias, implementación TDD y refactorizaciones acotadas.

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Cero Amnesia Operativa:** Cualquier nuevo agente o subagente que herede el workspace recupera el estado exacto de ejecución en $< 1\text{ ms}$ leyendo `working.md` sin depender del historial previo del chat.
- **Independencia Total del Host:** El núcleo en Rust (`alexandria`) permanece libre de acoplamiento a un IDE específico; los artefactos de `.agents/` y `mcp_config.json` actúan como adaptadores periféricos.
- **Continuidad en Ejecuciones Nocturnas:** Si la cuota del modelo principal expira a mitad de un sprint TDD, los subagentes continúan el ciclo `RED -> GREEN -> REFACTOR` desde el último hito asentado en `working.md`.

### Invariantes Constitucionales
- **Invariante M1:** Todo cambio de fase o conclusión de sprint TDD debe reflejarse atómicamente tanto en [[log]] como en `working.md`.
- **Invariante M2:** La concurrencia máxima de subagentes en el adaptador Antigravity queda fijada en $5$ para `architect` y $3$ para `builder`, respetando siempre la doble disyunción de [[topological-graph]].

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Contrato del Módulo:** [[host-adapters]]
- **Servidor MCP:** [[mcp-server]]
- **Persistencia SQLite:** [[database-schema]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-004-single-writer-mpsc-storage]]
