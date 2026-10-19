-- =============================================================================
-- ALEXANDRIA RUNTIME v2.4.0 — CANONICAL SQLITE WAL & FTS5 SCHEMA
-- Archivo: src/storage/schema.sql
-- =============================================================================

PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA mmap_size = 268435456;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -64000;

-- 1. Grafo Persistido de la Bóveda de Conocimiento (Plano 1 Relacional)
CREATE TABLE IF NOT EXISTS wiki_nodes (
    node_id TEXT PRIMARY KEY,
    relative_path TEXT NOT NULL UNIQUE,
    node_kind TEXT NOT NULL CHECK(node_kind IN ('Module', 'Architecture', 'ConstitutionalAdr', 'Skill', 'OperationalLog')),
    sha256_hex TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wiki_nodes_kind ON wiki_nodes(node_kind);

CREATE TABLE IF NOT EXISTS wiki_edges (
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    PRIMARY KEY (source_id, target_id),
    FOREIGN KEY (source_id) REFERENCES wiki_nodes(node_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_wiki_edges_target ON wiki_edges(target_id);

-- 2. Catálogo de Habilidades Procedimentales (Cápsulas JIT)
CREATE TABLE IF NOT EXISTS skills (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL,
    description TEXT NOT NULL,
    trigger_keywords_json TEXT NOT NULL,
    invariants_json TEXT NOT NULL,
    instructions_md TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_skills_domain ON skills(domain);

-- 3. Índice Léxico FTS5 para Skills (Contenido Externo + Tokenizador Porter)
CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(
    skill_id,
    domain,
    description,
    trigger_keywords_json,
    instructions_md,
    content='skills',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS skills_ai AFTER INSERT ON skills BEGIN
    INSERT INTO skills_fts(rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES (new.rowid, new.skill_id, new.domain, new.description, new.trigger_keywords_json, new.instructions_md);
END;

CREATE TRIGGER IF NOT EXISTS skills_ad AFTER DELETE ON skills BEGIN
    INSERT INTO skills_fts(skills_fts, rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES ('delete', old.rowid, old.skill_id, old.domain, old.description, old.trigger_keywords_json, old.instructions_md);
END;

CREATE TRIGGER IF NOT EXISTS skills_au AFTER UPDATE ON skills BEGIN
    INSERT INTO skills_fts(skills_fts, rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES ('delete', old.rowid, old.skill_id, old.domain, old.description, old.trigger_keywords_json, old.instructions_md);
    INSERT INTO skills_fts(rowid, skill_id, domain, description, trigger_keywords_json, instructions_md)
    VALUES (new.rowid, new.skill_id, new.domain, new.description, new.trigger_keywords_json, new.instructions_md);
END;

-- 4. Estado del Disyuntor Determinista (Circuit Breaker por Módulo)
CREATE TABLE IF NOT EXISTS circuit_breakers (
    module_name TEXT PRIMARY KEY,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL CHECK(state IN ('closed_ok', 'retry_allowed', 'tripped_abort')),
    last_failure_id TEXT,
    updated_at INTEGER NOT NULL
);

-- 5. Registro Forense de Incidentes (Nabu / FAIL-XXX)
CREATE TABLE IF NOT EXISTS forensic_incidents (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    failure_id TEXT NOT NULL UNIQUE,
    module_name TEXT NOT NULL,
    failed_gate TEXT NOT NULL,
    stack_trace TEXT NOT NULL,
    resolution_hint TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_forensic_incidents_module ON forensic_incidents(module_name, created_at DESC);

-- 6. Índice Léxico FTS5 para Incidentes Forenses (Previene SQLITE_CORRUPT_VTAB)
CREATE VIRTUAL TABLE IF NOT EXISTS incidents_fts USING fts5(
    failure_id,
    module_name,
    stack_trace,
    resolution_hint,
    content='forensic_incidents',
    content_rowid='rowid',
    tokenize='unicode61'
);

CREATE TRIGGER IF NOT EXISTS incidents_ai AFTER INSERT ON forensic_incidents BEGIN
    INSERT INTO incidents_fts(rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES (new.rowid, new.failure_id, new.module_name, new.stack_trace, new.resolution_hint);
END;

CREATE TRIGGER IF NOT EXISTS incidents_ad AFTER DELETE ON forensic_incidents BEGIN
    INSERT INTO incidents_fts(incidents_fts, rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES ('delete', old.rowid, old.failure_id, old.module_name, old.stack_trace, old.resolution_hint);
END;

CREATE TRIGGER IF NOT EXISTS incidents_au AFTER UPDATE ON forensic_incidents BEGIN
    INSERT INTO incidents_fts(incidents_fts, rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES ('delete', old.rowid, old.failure_id, old.module_name, old.stack_trace, old.resolution_hint);
    INSERT INTO incidents_fts(rowid, failure_id, module_name, stack_trace, resolution_hint)
    VALUES (new.rowid, new.failure_id, new.module_name, new.stack_trace, new.resolution_hint);
END;

-- 7. Historial Auditable de Veredictos de Compuertas de Calidad
CREATE TABLE IF NOT EXISTS quality_gate_verdicts (
    verdict_id INTEGER PRIMARY KEY AUTOINCREMENT,
    module_name TEXT NOT NULL,
    execution_path TEXT NOT NULL CHECK(execution_path IN ('alpha_fast', 'omega_pantheon')),
    tdd_phase TEXT NOT NULL CHECK(tdd_phase IN ('red_expect_assertion_failure', 'green_expect_pass', 'refactor_full_gate')),
    passed INTEGER NOT NULL CHECK(passed IN (0, 1)),
    diagnostics_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_verdicts_module_time ON quality_gate_verdicts(module_name, created_at DESC);
