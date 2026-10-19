---
id: ADR-002-prefix-pinned-caching
title: "ADR-002: Topología de Contexto Prefix-Pinned, Sub-Sesiones Efímeras y ADR Hoisting Transitivo"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - prefix-pinning
  - kv-cache
  - adr-hoisting
  - petgraph
---

# ADR-002: Topología de Contexto Prefix-Pinned, Sub-Sesiones Efímeras y ADR Hoisting Transitivo

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

Los motores de inferencia de Modelos de Lenguaje modernos implementan caché de claves y valores (*KV-Cache / Prompt Caching*) basada en coincidencia exacta de prefijos de tokens. Sin embargo, los frameworks multi-agente tradicionales destruyen sistemáticamente esta caché debido a tres prácticas:

1. **Mutación del System Prompt y Herramientas:** Inyectar instrucciones dinámicas, reglas de módulos o listas cambiantes de herramientas al inicio del prompt invalida el KV-Cache desde el primer token, multiplicando la latencia (*Time-To-First-Token*) y el costo por turno hasta en un $90\%$.
2. **Proliferación de Agentes Especializados Aislados:** Instanciar agentes distintos para Base de Datos, Backend y Frontend con prompts base diferentes impide reutilizar el bloque cacheado entre fases de una misma tarea.
3. **Poda Ingenua de Contexto vs. Volcado Total:** Inyectar toda la documentación del repositorio desborda la ventana de contexto; pero limitar la lectura estrictamente a los vecinos inmediatos ($\text{Depth}=1$) provoca que los módulos pierdan de vista las decisiones constitucionales (`[[ADR-XXX]]`) situadas a 2 o más saltos de distancia.

---

## 3. Decisión

Adoptamos una arquitectura de **Topología de Contexto con Prefijo Fijado (*Prefix Pinning*)** combinada con **ADR Hoisting Transitivo en RAM**:

1. **Inmutabilidad Bit a Bit del Prefijo Estático:**
   - El bloque inicial de la ventana de contexto (System Prompt Constitucional + Canon Compartido + Catálogo de las 6 Herramientas MCP en [[mcp-server]]) permanece **estrictamente inmutable bit a bit** durante toda la sesión.
   - Queda prohibido agregar, reordenar o eliminar herramientas de `tools/list` en tiempo de ejecución.
2. **Polimorfismo de `Enki` mediante *Forked Prefix Sub-Sessions*:**
   - El constructor `Enki` rota entre especialidades (Fase DB $\rightarrow$ Fase Backend $\rightarrow$ Fase Frontend) instanciando sub-sesiones efímeras que comparten el $100\%$ del prefijo estático en caché y anexan la especialización únicamente en el **sufijo dinámico volátil** mediante el retorno de las herramientas `prepare_task_context` y `resolve_skill_capsule`.
3. **Ecuación de Resolución Topológica con *ADR Hoisting* ($\text{Depth} \le 6$):**
   - El subsistema [[topological-graph]] resuelve el contexto en $\le 0.2\text{ ms}$ sobre `petgraph::stable_graph::StableDiGraph` aplicando:
     $$\text{Contexto}(M) = \{M\} \cup N_1^{\text{out}}(M) \cup \left\{ A \in \text{Reachable}_{\le 6}^{\text{out}}(M) \;\middle|\; \text{kind}(A) = \texttt{ConstitutionalAdr} \right\}$$
   - Los módulos ordinarios se restringen a profundidad $1$, mientras que todo nodo `ConstitutionalAdr` (`alexandria/decisions/ADR-*.md`) alcanzable hasta $6$ saltos (incluyendo cadenas `ADR-A -> ADR-B`) se eleva automáticamente a la ventana activa.

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Tasa de Acierto de KV-Cache $\ge 85\%$:** Reducción drástica de costos de inferencia y latencia entre sub-sesiones de fase.
- **Garantía Constitucional Sin Sobrecarga:** Los invariantes de los ADRs jamás se olvidan por distancia topológica, sin necesidad de cargar módulos intermedios irrelevantes ($\text{Depth} > 1$).

### Compromisos y Costos Asumidos
- Exige disciplina documental en `./alexandria/`: todo contrato de módulo debe enlazar explícitamente a sus vecinos y ADRs mediante enlaces bidireccionales `[[nodo]]`.

### Invariantes Constitucionales
- **Invariante P1:** El esquema JSON devuelto por `tools/list` en [[mcp-server]] es estático y determinista.
- **Invariante P2:** Ningún nodo de tipo `Module` o `Architecture` situado a distancia $> 1$ puede inyectarse automáticamente salvo solicitud explícita, mientras que todo `ConstitutionalAdr` a distancia $\le 6$ es de inyección obligatoria.

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Motor de Grafo:** [[topological-graph]]
- **Servidor MCP:** [[mcp-server]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-003-zero-trust-quality-gates]], [[ADR-004-single-writer-mpsc-storage]]
