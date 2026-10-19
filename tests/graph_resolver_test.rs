use alexandria::graph::resolver::{
    DisjunctionConflict, NodeKind, WikiNode, compute_sha256, resolve_context_with_adr_hoisting,
    verify_worktree_disjunction,
};
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use std::collections::HashMap;
use std::path::PathBuf;

fn make_node(id: &str, kind: NodeKind) -> WikiNode {
    let content = format!("# Node {}", id);
    WikiNode {
        id: id.to_string(),
        path: PathBuf::from(format!("wiki/{}.md", id)),
        kind,
        sha256: compute_sha256(&content),
        content,
    }
}

#[test]
fn test_strict_depth_one_for_modules() {
    let mut graph = StableDiGraph::<WikiNode, ()>::new();

    let mod_a = graph.add_node(make_node("core-engine", NodeKind::Module));
    let mod_b = graph.add_node(make_node("mcp-server", NodeKind::Module));
    let mod_c = graph.add_node(make_node("sandbox-runner", NodeKind::Module));
    let arch_db = graph.add_node(make_node("database-schema", NodeKind::Architecture));

    // core-engine (depth 0) -> mcp-server (depth 1) -> sandbox-runner (depth 2) -> database-schema (depth 3)
    graph.add_edge(mod_a, mod_b, ());
    graph.add_edge(mod_b, mod_c, ());
    graph.add_edge(mod_c, arch_db, ());

    let resolved = resolve_context_with_adr_hoisting(&graph, mod_a, 6);

    assert_eq!(resolved.target_module, "core-engine");
    assert_eq!(
        resolved.direct_neighbors,
        vec!["mcp-server".to_string()],
        "Only the direct neighbor at depth=1 must be injected"
    );
    assert!(
        !resolved
            .direct_neighbors
            .contains(&"sandbox-runner".to_string()),
        "Module at distance 2 must NOT be injected into direct_neighbors"
    );
    assert!(
        !resolved
            .direct_neighbors
            .contains(&"database-schema".to_string()),
        "Architecture node at distance 3 must NOT be injected into direct_neighbors"
    );
    assert!(resolved.hoisted_adrs.is_empty());
}

#[test]
fn test_transitive_adr_hoisting_chain() {
    // 1. Direct chain: ModuleA (0) -> ModuleB (1) -> ADR-002 (2) -> ADR-001 (3)
    let mut direct_graph = StableDiGraph::<WikiNode, ()>::new();
    let d_mod_a = direct_graph.add_node(make_node("ModuleA", NodeKind::Module));
    let d_mod_b = direct_graph.add_node(make_node("ModuleB", NodeKind::Module));
    let d_adr_002 = direct_graph.add_node(make_node(
        "ADR-002-prefix-pinned-caching",
        NodeKind::ConstitutionalAdr,
    ));
    let d_adr_001 = direct_graph.add_node(make_node(
        "ADR-001-kernel-space-harness",
        NodeKind::ConstitutionalAdr,
    ));

    direct_graph.add_edge(d_mod_a, d_mod_b, ());
    direct_graph.add_edge(d_mod_b, d_adr_002, ());
    direct_graph.add_edge(d_adr_002, d_adr_001, ());

    let res_direct = resolve_context_with_adr_hoisting(&direct_graph, d_mod_a, 6);
    assert_eq!(res_direct.target_module, "ModuleA");
    assert_eq!(
        res_direct.hoisted_adrs,
        vec![
            "ADR-002-prefix-pinned-caching".to_string(),
            "ADR-001-kernel-space-harness".to_string(),
        ]
    );
    assert!(!res_direct.hoisted_adrs.contains(&"ModuleB".to_string()));

    // 2. Chain with intermediate module at distance 2:
    // ModuleA (0) -> NeighborModule (1) -> ModuleB (2) -> ADR-002 (3) -> ADR-001 (4)
    let mut graph = StableDiGraph::<WikiNode, ()>::new();
    let mod_a = graph.add_node(make_node("ModuleA", NodeKind::Module));
    let mod_neighbor = graph.add_node(make_node("NeighborModule", NodeKind::Module));
    let mod_b = graph.add_node(make_node("ModuleB", NodeKind::Module));
    let adr_002 = graph.add_node(make_node(
        "ADR-002-prefix-pinned-caching",
        NodeKind::ConstitutionalAdr,
    ));
    let adr_001 = graph.add_node(make_node(
        "ADR-001-kernel-space-harness",
        NodeKind::ConstitutionalAdr,
    ));

    graph.add_edge(mod_a, mod_neighbor, ());
    graph.add_edge(mod_neighbor, mod_b, ());
    graph.add_edge(mod_b, adr_002, ());
    graph.add_edge(adr_002, adr_001, ());

    let resolved = resolve_context_with_adr_hoisting(&graph, mod_a, 6);

    assert_eq!(resolved.target_module, "ModuleA");
    assert_eq!(
        resolved.direct_neighbors,
        vec!["NeighborModule".to_string()]
    );
    assert!(
        !resolved.direct_neighbors.contains(&"ModuleB".to_string()),
        "Intermediate module ModuleB (depth=2) must NOT be included in direct context"
    );
    assert!(
        !resolved.hoisted_adrs.contains(&"ModuleB".to_string()),
        "ModuleB must never be classified as a hoisted ADR"
    );
    assert_eq!(
        resolved.hoisted_adrs,
        vec![
            "ADR-002-prefix-pinned-caching".to_string(),
            "ADR-001-kernel-space-harness".to_string(),
        ],
        "Both ADRs in the transitive chain (ADR-002 -> ADR-001) must be hoisted into ModuleA's context"
    );
}

#[test]
fn test_cycle_prevention_in_graph() {
    let mut graph = StableDiGraph::<WikiNode, ()>::new();

    let mod_a = graph.add_node(make_node("ModuleA", NodeKind::Module));
    let mod_b = graph.add_node(make_node("ModuleB", NodeKind::Module));
    let adr_003 = graph.add_node(make_node(
        "ADR-003-zero-trust-quality-gates",
        NodeKind::ConstitutionalAdr,
    ));

    // Directed cycle: ModuleA -> ModuleB -> ModuleA and outgoing link to ADR-003
    graph.add_edge(mod_a, mod_b, ());
    graph.add_edge(mod_b, mod_a, ());
    graph.add_edge(mod_b, adr_003, ());

    let resolved = resolve_context_with_adr_hoisting(&graph, mod_a, 6);

    assert_eq!(resolved.target_module, "ModuleA");
    assert_eq!(resolved.direct_neighbors, vec!["ModuleB".to_string()]);
    assert_eq!(
        resolved.hoisted_adrs,
        vec!["ADR-003-zero-trust-quality-gates".to_string()]
    );
}

#[test]
fn test_worktree_disjunction_checker() {
    let mut graph = StableDiGraph::<WikiNode, ()>::new();
    let mut node_map = HashMap::<String, NodeIndex>::new();

    let mod_auth = graph.add_node(make_node("auth-module", NodeKind::Module));
    let mod_billing = graph.add_node(make_node("billing-module", NodeKind::Module));
    let mod_shared_db = graph.add_node(make_node("shared-db", NodeKind::Module));
    let mod_ui_theme = graph.add_node(make_node("ui-theme", NodeKind::Module));
    let adr_001 = graph.add_node(make_node(
        "ADR-001-kernel-space-harness",
        NodeKind::ConstitutionalAdr,
    ));

    node_map.insert("auth-module".to_string(), mod_auth);
    node_map.insert("billing-module".to_string(), mod_billing);
    node_map.insert("shared-db".to_string(), mod_shared_db);
    node_map.insert("ui-theme".to_string(), mod_ui_theme);

    // auth-module and billing-module share direct neighbor shared-db (CONFLICT)
    graph.add_edge(mod_auth, mod_shared_db, ());
    graph.add_edge(mod_billing, mod_shared_db, ());

    // Both link to the same constitutional ADR (ALLOWED: ADRs are read-only)
    graph.add_edge(mod_auth, adr_001, ());
    graph.add_edge(mod_ui_theme, adr_001, ());

    let overlapping_tasks = vec![
        (
            "auth-module".to_string(),
            vec![PathBuf::from("src/auth/login.ts")],
        ),
        (
            "billing-module".to_string(),
            vec![PathBuf::from("src/billing/charge.ts")],
        ),
    ];

    let conflict_res = verify_worktree_disjunction(&graph, &node_map, &overlapping_tasks);
    assert_eq!(
        conflict_res,
        Err(DisjunctionConflict::WikiSubgraphOverlap {
            colliding_nodes: vec!["shared-db".to_string()],
        }),
        "Must reject parallel worktrees when two modules share a Depth=1 neighbor"
    );

    // Disjoint case: auth-module (with neighbor shared-db) vs ui-theme (no shared neighbors or files)
    let disjoint_tasks = vec![
        (
            "auth-module".to_string(),
            vec![PathBuf::from("src/auth/login.ts")],
        ),
        (
            "ui-theme".to_string(),
            vec![PathBuf::from("src/ui/tokens.ts")],
        ),
    ];

    let ok_res = verify_worktree_disjunction(&graph, &node_map, &disjoint_tasks);
    assert_eq!(
        ok_res,
        Ok(()),
        "Must approve parallel worktrees when both Depth=1 subgraphs and target files are disjoint"
    );
}
