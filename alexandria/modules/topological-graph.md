---
id: topological-graph
title: "Contrato de Subsistema: Motor Topológico en RAM (petgraph), ADR Hoisting y Disyunción de Worktrees"
type: module
status: canonical
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - module
  - petgraph
  - adr-hoisting
  - bfs
  - watcher
  - disjunction
---

# Contrato de Subsistema: Motor Topológico en RAM (petgraph), ADR Hoisting y Disyunción de Worktrees

> **Documento Canónico de Módulo:** `alexandria/modules/topological-graph.md`  
> **Archivo de Destino en Kernel:** `src/graph/resolver.rs`  
> **Decisiones Gobernantes:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-006-oxc-ast-engine]]

---

## 1. Modelo Matemático y Estructura del Grafo en Memoria

El subsistema **Topological Graph** mantiene en memoria RAM una representación dirigida y acotada de la bóveda `./alexandria/` empleando `petgraph::stable_graph::StableDiGraph<WikiNode, ()>`. Se elige `StableDiGraph` sobre `DiGraph` convencional porque garantiza que los índices de nodos (`NodeIndex`) permanezcan estables e invariantes ante eliminaciones o recargas incrementales disparadas por el *File Watcher*.

### Ecuación Fundamental de Resolución Topológica con *ADR Hoisting*

Para evitar tanto la ceguera contextual como la saturación de la ventana de atención (*Context Rot*), el contexto inyectado para un módulo objetivo $M$ se define formalmente como:

$$\text{Contexto}(M) = \{M\} \cup N_1^{\text{out}}(M) \cup \left\{ A \in \text{Reachable}_{\le 6}^{\text{out}}(M) \;\middle|\; \text{kind}(A) = \texttt{ConstitutionalAdr} \right\}$$

Donde:
- $N_1^{\text{out}}(M) = \{ V \in V(G) \mid (M, V) \in E(G) \}$ es el conjunto de vecinos directos a profundidad exactamente $1$.
- $\text{Reachable}_{\le 6}^{\text{out}}(M)$ es el conjunto de todos los nodos alcanzables desde $M$ mediante un camino dirigido de longitud $d \in \{1, 2, 3, 4, 5, 6\}$.
- **Propagación Transitiva de ADRs (`ADR-A -> ADR-B`):** El algoritmo BFS continúa explorando aristas salientes más allá de $\text{depth}=1$ (sin agregar nodos intermedios de tipo `Module` o `Architecture` al resultado) con el único propósito de capturar nodos `ConstitutionalAdr` enlazados transitivamente hasta 6 saltos de distancia.

```
   [ Módulo Objetivo: M ] (Depth = 0)
             │
             ├───► [ Módulo Vecino: V1 ] (Depth = 1) ──► Incluido en direct_neighbors
             │               │
             │               └───► [ Módulo Lejano: V2 ] (Depth = 2) ──► EXCLUIDO (Poda Depth=1)
             │                               │
             │                               └───► [[ ADR-001 ]] (Depth = 3) ──► HOISTED!
             │                                           │
             │                                           └───► [[ ADR-004 ]] (Depth = 4) ──► HOISTED!
             │
             └───► [[ ADR-003 ]] (Depth = 1) ──► HOISTED!
```

---

## 2. Sincronización Reactiva y Colapso de Ráfagas (*File Watcher Debounce*)

Los editores modernos (Obsidian, VS Code, Neovim) realizan guardados atómicos mediante secuencias de llamadas al sistema (`write temp file` $\rightarrow$ `fsync` $\rightarrow$ `rename` $\rightarrow$ `chmod`), generando entre 3 y 5 eventos de sistema de archivos en un intervalo de $5\text{ ms}$ a $20\text{ ms}$.

1. **Ventana de Colapso de 75 ms:** Se integra `notify-debouncer-mini` configurado con `Duration::from_millis(75)`. Toda ráfaga de eventos sobre `./alexandria/**/*.md` se colapsa en un único lote de rutas modificadas.
2. **Verificación Criptográfica SHA-256:** Antes de re-parsear los enlaces `[[nodo]]` o invalidar las aristas en `StableDiGraph`, el motor calcula el digest `sha2::Sha256` del archivo en disco y lo compara en $O(1)$ contra `node.sha256`. Si el hash coincide (por ejemplo, un evento `touch` o cambio de metadatos sin alteración de contenido), la actualización se descarta sin tocar el grafo.

---

## 3. Doble Invariante de Disyunción para *Git Worktrees* Paralelos

Cuando `Bel` evalúa instanciar múltiples constructores `Enki` en paralelo (`spawn_isolated_worktrees`), el motor verifica deterministamente en $\le 2\text{ ms}$ que para todo par de tareas $(A, B)$ con $A \neq B$ se cumplan dos condiciones de aislamiento:

1. **Disyunción Documental en `petgraph`:**
   $$\left( \{M_A\} \cup N_1(M_A) \right) \cap \left( \{M_B\} \cup N_1(M_B) \right) = \emptyset$$
   *(Los nodos `ConstitutionalAdr` son de solo lectura para `Enki`, por lo que pueden compartirse sin conflicto; los módulos y contratos en vecindad directa deben ser estrictamente disjuntos).*
2. **Disyunción de Código en `oxc_resolver`:**
   $$\text{Closure}_{\text{imports}}(F_A) \cap \text{Closure}_{\text{imports}}(F_B) = \emptyset$$

---

## 4. Implementación de Referencia en Rust (`src/graph/resolver.rs`)

```rust
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Module,
    Architecture,
    ConstitutionalAdr,
}

#[derive(Debug, Clone)]
pub struct WikiNode {
    pub id: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub content: String,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContext {
    pub target_module: String,
    pub direct_neighbors: Vec<String>,
    pub hoisted_adrs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisjunctionConflict {
    WikiSubgraphOverlap { colliding_nodes: Vec<String> },
    AstImportCollision { colliding_files: Vec<String> },
}

/// Calcula el hash SHA-256 de un bloque de texto para detección de cambios reales
pub fn compute_sha256(content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().into()
}

/// Extrae todos los enlaces bidireccionales de Obsidian `[[nodo]]` o `[[nodo|alias]]`
/// ignorando bloques de código cercados.
pub fn extract_wikilinks(markdown_text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let bytes = markdown_text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 3 < len {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i + 2;
            let mut end = start;
            while end + 1 < len && !(bytes[end] == b']' && bytes[end + 1] == b']') {
                if bytes[end] == b'\n' {
                    break;
                }
                end += 1;
            }
            if end + 1 < len && bytes[end] == b']' && bytes[end + 1] == b']' {
                if let Ok(raw_inner) = std::str::from_utf8(&bytes[start..end]) {
                    let target = raw_inner
                        .split('|')
                        .next()
                        .unwrap_or("")
                        .split('#')
                        .next()
                        .unwrap_or("")
                        .trim();
                    if !target.is_empty() && seen.insert(target.to_string()) {
                        links.push(target.to_string());
                    }
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }

    links
}

/// Resuelve el subgrafo de contexto aplicando vecindad estricta Depth=1 para nodos
/// ordinarios y elevación transitiva (ADR Hoisting) hasta `max_adr_depth` saltos
/// para nodos de tipo `ConstitutionalAdr`.
pub fn resolve_context_with_adr_hoisting(
    graph: &StableDiGraph<WikiNode, ()>,
    start_idx: NodeIndex,
    max_adr_depth: usize,
) -> ResolvedContext {
    let target_node = &graph[start_idx];
    let mut direct_neighbors = Vec::new();
    let mut hoisted_adrs = Vec::new();

    let mut visited = HashSet::new();
    visited.insert(start_idx);

    let mut queue = VecDeque::new();
    queue.push_back((start_idx, 0usize));

    while let Some((curr_idx, depth)) = queue.pop_front() {
        if depth >= max_adr_depth {
            continue;
        }

        for edge in graph.edges_directed(curr_idx, Direction::Outgoing) {
            let next_idx = edge.target();
            if !visited.insert(next_idx) {
                continue;
            }

            let next_node = &graph[next_idx];
            let next_depth = depth + 1;

            match (next_depth, &next_node.kind) {
                (1, NodeKind::ConstitutionalAdr) => {
                    hoisted_adrs.push(next_node.id.clone());
                    queue.push_back((next_idx, next_depth));
                }
                (1, _) => {
                    direct_neighbors.push(next_node.id.clone());
                    queue.push_back((next_idx, next_depth));
                }
                (_, NodeKind::ConstitutionalAdr) => {
                    hoisted_adrs.push(next_node.id.clone());
                    queue.push_back((next_idx, next_depth));
                }
                (_, _) => {
                    // No se añade a direct_neighbors (Depth > 1), pero se encola
                    // para permitir el descubrimiento transitivo de ADRs más profundos.
                    queue.push_back((next_idx, next_depth));
                }
            }
        }
    }

    ResolvedContext {
        target_module: target_node.id.clone(),
        direct_neighbors,
        hoisted_adrs,
    }
}

/// Verifica la doble invariante de disyunción antes de autorizar Git worktrees paralelos
pub fn verify_worktree_disjunction(
    graph: &StableDiGraph<WikiNode, ()>,
    node_map: &HashMap<String, NodeIndex>,
    tasks: &[(String, Vec<PathBuf>)], // (module_id, resolved_import_files)
) -> Result<(), DisjunctionConflict> {
    let mut claimed_wiki_nodes: HashSet<String> = HashSet::new();
    let mut colliding_wiki: Vec<String> = Vec::new();

    for (module_id, _) in tasks {
        if let Some(&idx) = node_map.get(module_id) {
            let resolved = resolve_context_with_adr_hoisting(graph, idx, 6);
            let mut active_scope = vec![resolved.target_module];
            active_scope.extend(resolved.direct_neighbors);

            for node_id in active_scope {
                if !claimed_wiki_nodes.insert(node_id.clone()) && !colliding_wiki.contains(&node_id) {
                    colliding_wiki.push(node_id);
                }
            }
        }
    }

    if !colliding_wiki.is_empty() {
        return Err(DisjunctionConflict::WikiSubgraphOverlap {
            colliding_nodes: colliding_wiki,
        });
    }

    let mut claimed_files: HashSet<PathBuf> = HashSet::new();
    let mut colliding_files: Vec<String> = Vec::new();

    for (_, files) in tasks {
        for file_path in files {
            if !claimed_files.insert(file_path.clone()) {
                let display = file_path.to_string_lossy().to_string();
                if !colliding_files.contains(&display) {
                    colliding_files.push(display);
                }
            }
        }
    }

    if !colliding_files.is_empty() {
        return Err(DisjunctionConflict::AstImportCollision { colliding_files });
    }

    Ok(())
}

/// Clasifica un archivo Markdown dentro de `./alexandria/` según su directorio padre
pub fn classify_node_kind(relative_path: &Path) -> NodeKind {
    let path_str = relative_path.to_string_lossy().replace('\\', "/");
    if path_str.contains("decisions/ADR-") || path_str.starts_with("decisions/") {
        NodeKind::ConstitutionalAdr
    } else if path_str.starts_with("architecture/") {
        NodeKind::Architecture
    } else {
        NodeKind::Module
    }
}
```

---

## 5. Grafo de Enlaces Canónicos

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Runtime Central:** [[core-engine]]
- **Servidor MCP:** [[mcp-server]]
- **Escáner AST y Resolver:** [[gate-guard-ast]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]], [[ADR-006-oxc-ast-engine]]
