---
id: host-adapters
title: "Contrato de Subsistema: Adaptadores Multi-Host (alexandria::adapters) y Puente de Persistencia working.md"
type: module
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - module
  - adapters
  - antigravity
  - mcp
  - working-md
  - persistence
---

# Contrato de Subsistema: Adaptadores Multi-Host (`alexandria::adapters`) y Puente de Persistencia `working.md`

> **Documento Canónico de Módulo:** `alexandria/modules/host-adapters.md`  
> **Artefactos Gobernados:** `.agents/**`, `working.md`, `mcp_config.json`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-007-multi-host-adapter-and-persistence]]

---

## 1. Arquitectura de Adaptación Multi-Host

El subsistema **Host Adapters** (`alexandria::adapters`) desacopla el núcleo determinista en Rust ([[core-engine]]) de las particularidades de cada entorno anfitrión (Google Antigravity 2.0, Claude Code, Cursor, Windsurf). Opera mediante tres componentes sincronizados:

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                       NÚCLEO DETERMINISTA: ALEXANDRIA CORE (RUST)                        │
│                 ~/.alexandria/vault.db (SQLite WAL) + Grafo en RAM (petgraph)            │
└──────────────┬───────────────────────────────────────┬───────────────────────────────────┘
               │                                       │
               ▼ (Proyección en Disco)                 ▼ (Transporte e Integración)
┌──────────────────────────────────────┐ ┌─────────────────────────────────────────────────┐
│    PUENTE DE PERSISTENCIA FÍSICA     │ │         CAPA DE ADAPTADORES (ADAPTERS)          │
│            working.md                │ ├─────────────────────────────────────────────────┤
│ • Estado de fases [x] / [ ]          │ │ 1. Adaptador Nativo Google Antigravity (.agents)│
│ • Último hito verificado             │ │    • architect.md (max_subagents: 5, planning)  │
│ • Siguiente acción TDD inmediata     │ │    • builder.md   (max_subagents: 3, fast)      │
│ • Política de fallback de cuota      │ │    • rules/quota-limits.md                      │
│                                      │ │    • skills/alexandria-vault/SKILL.md           │
│                                      │ │ 2. Adaptador Universal MCP (mcp_config.json)    │
│                                      │ │    • ./target/release/alexandria mcp (stdio)    │
└──────────────────────────────────────┘ └─────────────────────────────────────────────────┘
```

---

## 2. Puente de Persistencia Transaccional (`vault.db` $\leftrightarrow$ `working.md`)

Para impedir que la rotación de sesiones, cierres inesperados del IDE o límites de cuota borren el contexto operativo del agente, el puente de persistencia sincroniza el estado relacional de [[database-schema]] con el archivo plano `working.md` en la raíz del proyecto:

1. **Invariante de Lectura de Arranque:** Todo agente (`architect` o `builder`) debe leer `working.md` en el turno cero antes de invocar subprocesos o editar archivos.
2. **Invariante de Actualización Post-Gate:** Cada vez que `verify_and_gate_changes` o `finalize_task_and_log` completan una transición de fase (`Fase 0` $\rightarrow$ `Fase 1` $\rightarrow$ `Fase 2` $\rightarrow$ `Fase 3`), se actualiza atómicamente la lista de tareas en `working.md`.

---

## 3. Especificación del Adaptador Google Antigravity (`.agents/`)

### 3.1. Mapeo de Personalidades y Concurrencia Acotada
| Subagente en `.agents/agents/` | Arquetipo del Panteón | Modo de Ejecución | Límite de Iteraciones | Límite de Subagentes (`max_subagents`) | Restricción Constitucional |
|---|---|---|---|---|---|
| `architect.md` | `Bel` (Overseer) + `Utu` (Critic) | `--mode planning` | `30` | **`5`** | Prohibición absoluta de escribir código en `src/`. |
| `builder.md` | `Enki` (Builder Polimórfico) | `--mode fast` | `40` | **`3`** | Obligación de ciclo TDD (`RED -> GREEN -> REFACTOR`) y cero `any`. |

### 3.2. Política de Cuota y Ventanas Móviles (`.agents/rules/quota-limits.md`)
- **Fase de Gobierno y Diseño (`--mode planning`):** Reservada exclusivamente para el diseño de ADRs en `alexandria/decisions/`, contratos en `alexandria/modules/` y auditorías SAST en frío.
- **Fase de Construcción y Pruebas (`--mode fast`):** Activada obligatoriamente para la generación masiva de código Rust/TypeScript, suites de pruebas y refactorizaciones, preservando la ventana móvil de 5 horas de los modelos de alto razonamiento.

### 3.3. Carga Progresiva de Contexto (`.agents/skills/alexandria-vault/SKILL.md`)
- Prohíbe la lectura indiscriminada de toda la carpeta `alexandria/`.
- Obliga a consultar primero [[index]] y cargar únicamente el contrato del módulo objetivo ($\text{Depth}=1$) junto con sus `[[ADR-XXX]]` directamente vinculados.

---

## 4. Especificación del Adaptador Universal MCP (`mcp_config.json`)

Expone el binario compilado en modo `release` (`./target/release/alexandria`) pasando el argumento `mcp`, lo que activa el bucle `run_mcp_stdio_loop()` definido en [[mcp-server]]:

```json
{
  "mcpServers": {
    "alexandria": {
      "command": "./target/release/alexandria",
      "args": ["mcp"]
    }
  }
}
```

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Runtime Central:** [[core-engine]]
- **Servidor MCP:** [[mcp-server]]
- **Motor Topológico:** [[topological-graph]]
- **Esquema de Base de Datos:** [[database-schema]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-004-single-writer-mpsc-storage]], [[ADR-007-multi-host-adapter-and-persistence]]
- **Bitácora de Cambios:** [[log]]
