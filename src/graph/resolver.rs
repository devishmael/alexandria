use crate::runner::vitest_sandbox::clean_canonical_path;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_resolver::{ResolveOptions, Resolver};
use oxc_span::SourceType;
use petgraph::Direction;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

pub fn compute_sha256(content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().into()
}

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

pub fn verify_worktree_disjunction(
    graph: &StableDiGraph<WikiNode, ()>,
    node_map: &HashMap<String, NodeIndex>,
    tasks: &[(String, Vec<PathBuf>)],
) -> Result<(), DisjunctionConflict> {
    let mut claimed_wiki_nodes: HashSet<String> = HashSet::new();
    let mut colliding_wiki: Vec<String> = Vec::new();

    for (module_id, _) in tasks {
        if let Some(&idx) = node_map.get(module_id) {
            let resolved = resolve_context_with_adr_hoisting(graph, idx, 6);
            let mut active_scope = vec![resolved.target_module];
            active_scope.extend(resolved.direct_neighbors);

            for node_id in active_scope {
                if !claimed_wiki_nodes.insert(node_id.clone())
                    && !colliding_wiki.contains(&node_id)
                {
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
        let task_footprint = resolve_mutable_import_footprint(files);
        for file_path in task_footprint {
            if !claimed_files.insert(file_path.clone()) {
                let display = file_path.to_string_lossy().to_string();
                if !colliding_files.contains(&display) {
                    colliding_files.push(display);
                }
            }
        }
    }

    if !colliding_files.is_empty() {
        colliding_files.sort();
        return Err(DisjunctionConflict::AstImportCollision { colliding_files });
    }

    Ok(())
}

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

fn collect_markdown_files_recursively(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "private" {
                collect_markdown_files_recursively(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

pub fn load_vault_graph_from_dir(
    vault_dir: &Path,
) -> (StableDiGraph<WikiNode, ()>, HashMap<String, NodeIndex>) {
    let mut graph = StableDiGraph::new();
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

    if !vault_dir.exists() {
        return (graph, node_map);
    }

    let mut md_files = Vec::new();
    collect_markdown_files_recursively(vault_dir, &mut md_files);
    md_files.sort();

    let mut pending_links: Vec<(NodeIndex, Vec<String>)> = Vec::new();

    for file_path in md_files {
        let Ok(content) = std::fs::read_to_string(&file_path) else {
            continue;
        };
        let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let node_id = stem.to_string();
        let rel_path = file_path.strip_prefix(vault_dir).unwrap_or(&file_path);
        let kind = classify_node_kind(rel_path);
        let sha256 = compute_sha256(&content);
        let links = extract_wikilinks(&content);

        let idx = graph.add_node(WikiNode {
            id: node_id.clone(),
            path: file_path,
            kind,
            content,
            sha256,
        });
        node_map.insert(node_id, idx);
        pending_links.push((idx, links));
    }

    for (src_idx, links) in pending_links {
        for target_id in links {
            if let Some(&dst_idx) = node_map.get(&target_id) {
                if src_idx != dst_idx && !graph.contains_edge(src_idx, dst_idx) {
                    graph.add_edge(src_idx, dst_idx, ());
                }
            }
        }
    }

    (graph, node_map)
}

pub fn resolve_mutable_import_footprint(files: &[PathBuf]) -> HashSet<PathBuf> {
    let options = ResolveOptions {
        extensions: vec![
            ".ts".into(),
            ".tsx".into(),
            ".js".into(),
            ".jsx".into(),
            ".json".into(),
        ],
        ..ResolveOptions::default()
    };
    let resolver = Resolver::new(options);

    let mut footprint: HashSet<PathBuf> = HashSet::new();

    for raw_path in files {
        let normalized_self = if raw_path.exists() {
            clean_canonical_path(raw_path).unwrap_or_else(|_| raw_path.clone())
        } else {
            raw_path.clone()
        };
        footprint.insert(normalized_self.clone());

        if !normalized_self.is_file() {
            continue;
        }
        let Ok(source_text) = std::fs::read_to_string(&normalized_self) else {
            continue;
        };
        let Some(parent_dir) = normalized_self.parent() else {
            continue;
        };

        let ext = normalized_self
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("ts");
        let is_tsx = ext.eq_ignore_ascii_case("tsx") || ext.eq_ignore_ascii_case("jsx");

        let allocator = Allocator::default();
        let source_type = SourceType::default().with_typescript(true).with_jsx(is_tsx);
        let parsed = Parser::new(&allocator, &source_text, source_type).parse();

        for stmt in &parsed.program.body {
            let specifier_opt = match stmt {
                Statement::ImportDeclaration(imp) => Some(imp.source.value.as_str()),
                Statement::ExportNamedDeclaration(exp) => {
                    exp.source.as_ref().map(|s| s.value.as_str())
                }
                Statement::ExportAllDeclaration(exp_all) => Some(exp_all.source.value.as_str()),
                _ => None,
            };

            if let Some(spec) = specifier_opt {
                if let Ok(resolution) = resolver.resolve(parent_dir, spec) {
                    let full_p = resolution.full_path();
                    let resolved_path = clean_canonical_path(&full_p).unwrap_or(full_p);
                    footprint.insert(resolved_path);
                }
            }
        }
    }

    footprint
}

pub fn serialize_graph_for_sqlite(
    vault_dir: &Path,
    graph: &StableDiGraph<WikiNode, ()>,
) -> (Vec<(String, String, String, String)>, Vec<(String, String)>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for idx in graph.node_indices() {
        let node = &graph[idx];
        let rel = node
            .path
            .strip_prefix(vault_dir)
            .unwrap_or(&node.path)
            .to_string_lossy()
            .replace('\\', "/");
        let kind_str = match node.kind {
            NodeKind::Module => "Module",
            NodeKind::Architecture => "Architecture",
            NodeKind::ConstitutionalAdr => "ConstitutionalAdr",
        };
        let mut hex_hash = String::with_capacity(64);
        for b in node.sha256 {
            use std::fmt::Write;
            let _ = write!(&mut hex_hash, "{:02x}", b);
        }
        nodes.push((node.id.clone(), rel, kind_str.to_string(), hex_hash));
    }

    for edge in graph.edge_references() {
        let src_id = graph[edge.source()].id.clone();
        let dst_id = graph[edge.target()].id.clone();
        edges.push((src_id, dst_id));
    }

    (nodes, edges)
}

pub fn start_vault_debounced_watcher(
    vault_dir: PathBuf,
    debounce_ms: u64,
) -> Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>, String> {
    std::fs::create_dir_all(&vault_dir)
        .map_err(|e| format!("Failed to create vault directory for watcher {:?}: {}", vault_dir, e))?;

    let watched_root = vault_dir.clone();
    let mut debouncer = notify_debouncer_mini::new_debouncer(
        Duration::from_millis(debounce_ms),
        move |res: notify_debouncer_mini::DebounceEventResult| {
            if let Ok(events) = res {
                let should_reload = events.iter().any(|ev| {
                    let rel = ev.path.strip_prefix(&watched_root).unwrap_or(&ev.path);
                    let in_ignored = rel.components().any(|c| {
                        let s = c.as_os_str().to_string_lossy();
                        s.starts_with('.') || s == "private"
                    });
                    !in_ignored && ev.path.extension().and_then(|e| e.to_str()) == Some("md")
                });
                if should_reload {
                    let _ = load_vault_graph_from_dir(&watched_root);
                }
            }
        },
    )
    .map_err(|e| format!("Failed to initialize 75ms debounced watcher: {}", e))?;

    debouncer
        .watcher()
        .watch(&vault_dir, notify::RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch vault directory {:?}: {}", vault_dir, e))?;

    Ok(debouncer)
}
