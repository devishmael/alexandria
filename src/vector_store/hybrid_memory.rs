use crate::storage::actor::{VaultStorageHandle, build_safe_fts5_or_query};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HybridMemoryQueryResult {
    pub chunk_id: String,
    pub source_kind: String,
    pub agent_id: String,
    pub module_name: String,
    pub compact_summary: String,
    pub token_estimate: usize,
    pub rrf_score: f32,
}

fn fnv1a_hash64(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = 0xcbf29ce484222325u64 ^ seed;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn light_stem(token: &str) -> &str {
    for suffix in ["ation", "tions", "tion", "ments", "ment", "ing", "ed", "es", "s"] {
        if token.len() > suffix.len() + 3 && token.ends_with(suffix) {
            return &token[..token.len() - suffix.len()];
        }
    }
    token
}

/// Computes a deterministic 64-dimensional L2-normalized `int8` semantic embedding (`[-127, 127]`)
/// via multi-granular lexical and character n-gram feature hashing with zero external model weights.
pub fn compute_deterministic_int8_embedding(text: &str) -> [i8; 64] {
    let mut accum = [0.0f32; 64];

    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| s.len() >= 2)
        .collect();

    if tokens.is_empty() {
        return [0i8; 64];
    }

    for (idx, tok) in tokens.iter().enumerate() {
        let stem = light_stem(tok);

        // 1. Unigram stem projection (weight 2.0)
        let h1 = fnv1a_hash64(stem.as_bytes(), 0x101);
        let dim1 = (h1 as usize) % 64;
        let sign1 = if (h1 >> 8) & 1 == 0 { 1.0 } else { -1.0 };
        accum[dim1] += 2.0 * sign1;

        // 2. Exact token projection (weight 1.2)
        let h2 = fnv1a_hash64(tok.as_bytes(), 0x202);
        let dim2 = (h2 as usize) % 64;
        let sign2 = if (h2 >> 8) & 1 == 0 { 1.0 } else { -1.0 };
        accum[dim2] += 1.2 * sign2;

        // 3. Word bigram projection (weight 1.5)
        if idx + 1 < tokens.len() {
            let bigram = format!("{}_{}", stem, light_stem(&tokens[idx + 1]));
            let hb = fnv1a_hash64(bigram.as_bytes(), 0x303);
            let dim_b = (hb as usize) % 64;
            let sign_b = if (hb >> 8) & 1 == 0 { 1.0 } else { -1.0 };
            accum[dim_b] += 1.5 * sign_b;
        }

        // 4. Character trigram projections for morphological resilience (weight 0.5)
        let bytes = tok.as_bytes();
        if bytes.len() >= 3 {
            for window in bytes.windows(3) {
                let hc = fnv1a_hash64(window, 0x404);
                let dim_c = (hc as usize) % 64;
                let sign_c = if (hc >> 8) & 1 == 0 { 1.0 } else { -1.0 };
                accum[dim_c] += 0.5 * sign_c;
            }
        }
    }

    let norm_sq: f32 = accum.iter().map(|v| v * v).sum();
    if norm_sq <= f32::EPSILON {
        return [0i8; 64];
    }

    let norm = norm_sq.sqrt();
    let mut quantized = [0i8; 64];
    for i in 0..64 {
        let unit = (accum[i] / norm).clamp(-1.0, 1.0);
        quantized[i] = (unit * 127.0).round() as i8;
    }
    quantized
}

/// Computes exact cosine similarity in `[-1.0, 1.0]` between two 64-dimensional `int8` vectors.
pub fn cosine_similarity_int8(a: &[i8; 64], b: &[i8; 64]) -> f32 {
    let mut dot: i32 = 0;
    let mut norm_a: i32 = 0;
    let mut norm_b: i32 = 0;

    for i in 0..64 {
        let va = a[i] as i32;
        let vb = b[i] as i32;
        dot += va * vb;
        norm_a += va * va;
        norm_b += vb * vb;
    }

    if norm_a == 0 || norm_b == 0 {
        return 0.0;
    }

    (dot as f32) / ((norm_a as f32).sqrt() * (norm_b as f32).sqrt())
}

/// Distills raw user or agent text into a high-density atomic micro-capsule (`<= 60` tokens)
/// and returns `(compact_summary, token_estimate)`.
pub fn distill_into_micro_chunk(raw_text: &str, max_tokens: usize) -> (String, usize) {
    let effective_cap = max_tokens.clamp(8, 60);
    let words: Vec<&str> = raw_text.split_whitespace().collect();

    if words.is_empty() {
        return (String::new(), 0);
    }

    let max_words = ((effective_cap * 3) / 4).max(1);
    let selected_words = if words.len() > max_words {
        &words[..max_words]
    } else {
        &words[..]
    };

    let summary = selected_words.join(" ");
    let estimated_tokens = ((selected_words.len() * 4).div_ceil(3)).min(effective_cap);
    (summary, estimated_tokens.max(1))
}

/// Combines `int8` cosine similarity ranking with SQLite FTS5 BM25 ranking using
/// Reciprocal Rank Fusion (`RRF = 1/(60 + rank_vec) + 1/(60 + rank_bm25)`) under a strict token budget.
pub fn query_hybrid_vector_memory(
    storage: &VaultStorageHandle,
    query: &str,
    max_token_budget: usize,
) -> Result<Vec<HybridMemoryQueryResult>, String> {
    let conn = Connection::open_with_flags(storage.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open vault.db for hybrid vector query: {}", e))?;

    let query_vec = compute_deterministic_int8_embedding(query);

    struct CandidateRow {
        chunk_id: String,
        source_kind: String,
        agent_id: String,
        module_name: String,
        compact_summary: String,
        token_estimate: usize,
        cosine_sim: f32,
    }

    let mut stmt = conn
        .prepare(
            "SELECT chunk_id, source_kind, agent_id, module_name, compact_summary, token_estimate, embedding_int8_json
             FROM episodic_vector_memory
             ORDER BY created_at DESC LIMIT 256",
        )
        .map_err(|e| e.to_string())?;

    let mut candidates: Vec<CandidateRow> = stmt
        .query_map([], |row| {
            let chunk_id: String = row.get(0)?;
            let source_kind: String = row.get(1)?;
            let agent_id: String = row.get(2)?;
            let module_name: String = row.get(3)?;
            let compact_summary: String = row.get(4)?;
            let token_estimate: i64 = row.get(5)?;
            let emb_json: String = row.get(6)?;
            let parsed_vec: Vec<i8> = serde_json::from_str(&emb_json).unwrap_or_default();
            let mut arr = [0i8; 64];
            if parsed_vec.len() == 64 {
                arr.copy_from_slice(&parsed_vec);
            }
            let cosine_sim = cosine_similarity_int8(&query_vec, &arr);
            Ok(CandidateRow {
                chunk_id,
                source_kind,
                agent_id,
                module_name,
                compact_summary,
                token_estimate: token_estimate.max(1) as usize,
                cosine_sim,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Sort by cosine similarity descending to assign vector ranks (1-based)
    candidates.sort_by(|a, b| {
        b.cosine_sim
            .partial_cmp(&a.cosine_sim)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut vec_ranks: HashMap<String, usize> = HashMap::with_capacity(candidates.len());
    for (idx, c) in candidates.iter().enumerate() {
        vec_ranks.insert(c.chunk_id.clone(), idx + 1);
    }

    // Query FTS5 BM25 lexical ranks (1-based)
    let mut bm25_ranks: HashMap<String, usize> = HashMap::new();
    if let Some(safe_fts_query) = build_safe_fts5_or_query(query) {
        if let Ok(mut fts_stmt) = conn.prepare(
            "SELECT chunk_id FROM episodic_memory_fts WHERE episodic_memory_fts MATCH ?1 ORDER BY bm25(episodic_memory_fts) LIMIT 64",
        ) {
            if let Ok(rows) = fts_stmt.query_map(params![safe_fts_query], |r| r.get::<_, String>(0)) {
                for (idx, id_res) in rows.flatten().enumerate() {
                    bm25_ranks.insert(id_res, idx + 1);
                }
            }
        }
    }

    let mut scored: Vec<HybridMemoryQueryResult> = candidates
        .into_iter()
        .map(|c| {
            let r_vec = *vec_ranks.get(&c.chunk_id).unwrap_or(&1000) as f32;
            let r_bm25 = bm25_ranks
                .get(&c.chunk_id)
                .map(|&r| 1.0 / (60.0 + r as f32))
                .unwrap_or(0.0);
            let rrf_score = (1.0 / (60.0 + r_vec)) + r_bm25 + (c.cosine_sim.max(0.0) * 0.01);

            HybridMemoryQueryResult {
                chunk_id: c.chunk_id,
                source_kind: c.source_kind,
                agent_id: c.agent_id,
                module_name: c.module_name,
                compact_summary: c.compact_summary,
                token_estimate: c.token_estimate,
                rrf_score,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Enforce strict max_token_budget
    let mut selected = Vec::new();
    let mut used_tokens = 0usize;
    for item in scored {
        if used_tokens + item.token_estimate <= max_token_budget {
            used_tokens += item.token_estimate;
            selected.push(item);
        }
    }

    Ok(selected)
}

/// Exports the current episodic vector memory table to `.alexandria/vector-store/memory-index.json`.
pub fn export_vector_store_snapshot(
    storage: &VaultStorageHandle,
    vector_dir: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(vector_dir)
        .map_err(|e| format!("Failed to create vector store directory {:?}: {}", vector_dir, e))?;

    let conn = Connection::open_with_flags(storage.db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT chunk_id, source_kind, agent_id, module_name, tags_json, compact_summary, token_estimate, embedding_int8_json, created_at
             FROM episodic_vector_memory
             ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let entries: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let chunk_id: String = row.get(0)?;
            let source_kind: String = row.get(1)?;
            let agent_id: String = row.get(2)?;
            let module_name: String = row.get(3)?;
            let tags_raw: String = row.get(4)?;
            let compact_summary: String = row.get(5)?;
            let token_estimate: i64 = row.get(6)?;
            let emb_raw: String = row.get(7)?;
            let created_at: i64 = row.get(8)?;

            let tags: serde_json::Value = serde_json::from_str(&tags_raw).unwrap_or_else(|_| json!([]));
            let embedding: serde_json::Value =
                serde_json::from_str(&emb_raw).unwrap_or_else(|_| json!([]));

            Ok(json!({
                "chunkId": chunk_id,
                "sourceKind": source_kind,
                "agentId": agent_id,
                "moduleName": module_name,
                "tags": tags,
                "compactSummary": compact_summary,
                "tokenEstimate": token_estimate,
                "embeddingInt8": embedding,
                "createdAt": created_at
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let payload = json!({
        "schemaVersion": "alexandria.vector_store.v2.5",
        "dimensions": 64,
        "quantization": "int8_l2_normalized",
        "fusionAlgorithm": "reciprocal_rank_fusion_k60",
        "totalChunks": entries.len(),
        "chunks": entries
    });

    let index_path = vector_dir.join("memory-index.json");
    let serialized = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(&index_path, serialized)
        .map_err(|e| format!("Failed to write {:?}: {}", index_path, e))?;

    Ok(index_path)
}
