use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const CANONICAL_CAPSULE_SCHEMA_VERSION: &str = "alexandria.capsule.v2.4";
pub const MANIFEST_FILENAME: &str = "alexandria-capsules.manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlexandriaHookSpec {
    pub event: String,
    pub action: String,
    #[serde(alias = "target_filter")]
    pub target_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlexandriaCapsuleJson {
    #[serde(default = "default_schema_version", alias = "schema_version")]
    pub schema_version: String,
    #[serde(alias = "skill_id", alias = "id")]
    pub skill_id: String,
    pub domain: String,
    #[serde(default = "default_profiles", alias = "profile_compatibility")]
    pub profile_compatibility: Vec<String>,
    pub description: String,
    #[serde(default, alias = "trigger_keywords")]
    pub trigger_keywords: Vec<String>,
    #[serde(default = "default_required_tools", alias = "required_tools")]
    pub required_tools: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<AlexandriaHookSpec>,
    #[serde(default, alias = "instructions_markdown")]
    pub instructions_markdown: String,
    #[serde(default, alias = "content_sha256")]
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlexandriaCapsuleManifestEntry {
    pub skill_id: String,
    pub domain: String,
    pub file_name: String,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlexandriaCapsuleManifest {
    pub schema_version: String,
    pub total_capsules: usize,
    pub capsules: Vec<AlexandriaCapsuleManifestEntry>,
}

fn default_schema_version() -> String {
    CANONICAL_CAPSULE_SCHEMA_VERSION.to_string()
}

fn default_profiles() -> Vec<String> {
    vec![
        "mvp-fullstack-ts".to_string(),
        "enterprise-microservices".to_string(),
    ]
}

fn default_required_tools() -> Vec<String> {
    vec![
        "prepare_task_context".to_string(),
        "verify_and_gate_changes".to_string(),
    ]
}

pub fn compute_sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    let mut hex_out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex_out, "{:02x}", b);
    }
    hex_out
}

pub fn sanitize_skill_id_for_filename(skill_id: &str) -> String {
    skill_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn slugify_identifier(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == ':' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn infer_domain_from_text(skill_name: &str, description: &str, body: &str) -> String {
    let combined = format!(
        "{} {} {}",
        skill_name.to_lowercase(),
        description.to_lowercase(),
        body.to_lowercase()
    );
    if combined.contains("vitest")
        || combined.contains("tdd")
        || combined.contains("test")
        || combined.contains("assertion")
    {
        "testing".to_string()
    } else if combined.contains("prisma")
        || combined.contains("database")
        || combined.contains("sql")
        || combined.contains("sqlite")
        || combined.contains("transaction")
    {
        "database".to_string()
    } else if combined.contains("security")
        || combined.contains("sast")
        || combined.contains("secret")
        || combined.contains("credential")
        || combined.contains("utu")
    {
        "security".to_string()
    } else if combined.contains("react")
        || combined.contains("frontend")
        || combined.contains("component")
        || combined.contains("tailwind")
    {
        "frontend".to_string()
    } else {
        "backend".to_string()
    }
}

fn infer_trigger_keywords(skill_id: &str, description: &str) -> Vec<String> {
    let stopwords: HashSet<&str> = [
        "the", "and", "for", "with", "from", "into", "that", "this", "must", "only", "when",
        "enforces", "strict", "rules", "skill", "capsule",
    ]
    .into_iter()
    .collect();

    let mut keywords: Vec<String> = Vec::new();
    let combined = format!("{} {}", skill_id, description);
    for token in combined
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| s.len() >= 3)
    {
        if stopwords.contains(token.as_str()) {
            continue;
        }
        if !keywords.contains(&token) {
            keywords.push(token);
        }
        if keywords.len() >= 10 {
            break;
        }
    }

    if keywords.is_empty() {
        keywords.push("general".to_string());
    }
    keywords
}

fn parse_hook_bullet_line(line: &str) -> Option<AlexandriaHookSpec> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Format: "event: action (target_filter)" or "event -> action"
    let (left, right) = if let Some(pair) = trimmed.split_once("->") {
        pair
    } else if let Some(pair) = trimmed.split_once(':') {
        pair
    } else {
        return Some(AlexandriaHookSpec {
            event: "pre_execution".to_string(),
            action: trimmed.to_string(),
            target_filter: None,
        });
    };

    let event = left.trim().trim_matches('`').to_string();
    let right_trimmed = right.trim();

    if let Some((act_part, filter_part)) = right_trimmed.split_once('(') {
        let action = act_part.trim().trim_matches('`').to_string();
        let filter = filter_part
            .trim_end_matches(')')
            .trim()
            .trim_matches('`')
            .to_string();
        Some(AlexandriaHookSpec {
            event,
            action,
            target_filter: if filter.is_empty() {
                None
            } else {
                Some(filter)
            },
        })
    } else {
        Some(AlexandriaHookSpec {
            event,
            action: right_trimmed.trim_matches('`').to_string(),
            target_filter: None,
        })
    }
}

fn extract_markdown_sections(
    markdown_body: &str,
) -> (Vec<String>, Vec<AlexandriaHookSpec>, Vec<String>) {
    #[derive(PartialEq, Eq)]
    enum ActiveSection {
        None,
        Invariants,
        Hooks,
        Tools,
    }

    let parser = Parser::new(markdown_body);
    let mut section = ActiveSection::None;
    let mut in_heading = false;
    let mut heading_buf = String::new();
    let mut in_item = false;
    let mut item_buf = String::new();

    let mut invariants = Vec::new();
    let mut hooks = Vec::new();
    let mut tools = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading_buf.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let h_lower = heading_buf.trim().to_ascii_lowercase();
                if h_lower.contains("invariant") || h_lower.contains("rule") {
                    section = ActiveSection::Invariants;
                } else if h_lower.contains("hook") {
                    section = ActiveSection::Hooks;
                } else if h_lower.contains("tool") {
                    section = ActiveSection::Tools;
                } else {
                    section = ActiveSection::None;
                }
            }
            Event::Start(Tag::Item) => {
                if section != ActiveSection::None {
                    in_item = true;
                    item_buf.clear();
                }
            }
            Event::End(TagEnd::Item) => {
                if in_item {
                    in_item = false;
                    let text = item_buf.trim().to_string();
                    if !text.is_empty() {
                        match section {
                            ActiveSection::Invariants => invariants.push(text),
                            ActiveSection::Hooks => {
                                if let Some(h) = parse_hook_bullet_line(&text) {
                                    hooks.push(h);
                                }
                            }
                            ActiveSection::Tools => tools.push(text),
                            ActiveSection::None => {}
                        }
                    }
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if in_heading {
                    heading_buf.push_str(&t);
                } else if in_item {
                    item_buf.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_item {
                    item_buf.push(' ');
                }
            }
            _ => {}
        }
    }

    (invariants, hooks, tools)
}

fn split_frontmatter_and_body(raw_md: &str) -> (Option<&str>, &str) {
    let trimmed = raw_md.trim_start_matches('\u{feff}');
    if let Some(after_open) = trimmed
        .strip_prefix("---\r\n")
        .or_else(|| trimmed.strip_prefix("---\n"))
    {
        if let Some(end_idx) = after_open.find("\n---") {
            let yaml_part = &after_open[..end_idx];
            let rest = &after_open[end_idx + 4..];
            let body = rest
                .strip_prefix("\r\n")
                .or_else(|| rest.strip_prefix('\n'))
                .unwrap_or(rest);
            return (Some(yaml_part), body);
        }
    }
    (None, trimmed)
}

fn extract_string_array_from_value(val: Option<&Value>) -> Vec<String> {
    match val {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) => s
            .split(',')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn extract_hooks_from_value(val: Option<&Value>) -> Vec<AlexandriaHookSpec> {
    let Some(Value::Array(arr)) = val else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in arr {
        if let Some(obj) = item.as_object() {
            let event = obj
                .get("event")
                .or_else(|| obj.get("trigger"))
                .or_else(|| obj.get("phase"))
                .and_then(|v| v.as_str())
                .unwrap_or("pre_execution")
                .to_string();
            let action = obj
                .get("action")
                .or_else(|| obj.get("command"))
                .or_else(|| obj.get("handler"))
                .and_then(|v| v.as_str())
                .unwrap_or("enforce_invariant")
                .to_string();
            let target_filter = obj
                .get("targetFilter")
                .or_else(|| obj.get("target_filter"))
                .or_else(|| obj.get("filter"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            out.push(AlexandriaHookSpec {
                event,
                action,
                target_filter,
            });
        } else if let Some(s) = item.as_str() {
            if let Some(h) = parse_hook_bullet_line(s) {
                out.push(h);
            }
        }
    }
    out
}

fn infer_fallback_name_from_path(file_path: &Path) -> String {
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed-skill");
    if stem.eq_ignore_ascii_case("skill") {
        if let Some(parent_name) = file_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty() && *n != ".")
        {
            return slugify_identifier(parent_name);
        }
    }
    slugify_identifier(stem)
}

fn build_capsule_from_metadata_and_body(
    meta: &Value,
    markdown_body: &str,
    file_path: &Path,
) -> AlexandriaCapsuleJson {
    let normalized_body = markdown_body.replace("\r\n", "\n").trim().to_string();
    let (md_invariants, md_hooks, md_tools) = extract_markdown_sections(&normalized_body);

    let raw_id = meta
        .get("id")
        .or_else(|| meta.get("skillId"))
        .or_else(|| meta.get("skill_id"))
        .or_else(|| meta.get("name"))
        .and_then(|v| v.as_str())
        .map(slugify_identifier)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| infer_fallback_name_from_path(file_path));

    let description = meta
        .get("description")
        .or_else(|| meta.get("summary"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            normalized_body
                .lines()
                .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
                .unwrap_or("Refactored Alexandria procedural skill capsule.")
                .trim()
                .to_string()
        });

    let domain = meta
        .get("domain")
        .or_else(|| meta.get("category"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| infer_domain_from_text(&raw_id, &description, &normalized_body));

    let skill_id = if raw_id.contains(':') {
        raw_id
    } else {
        format!("{}:{}", domain, raw_id)
    };

    let mut profile_compatibility = extract_string_array_from_value(
        meta.get("profile_compatibility")
            .or_else(|| meta.get("profileCompatibility"))
            .or_else(|| meta.get("profiles")),
    );
    if profile_compatibility.is_empty() {
        profile_compatibility = default_profiles();
    }

    let mut trigger_keywords = extract_string_array_from_value(
        meta.get("trigger_keywords")
            .or_else(|| meta.get("triggerKeywords"))
            .or_else(|| meta.get("keywords"))
            .or_else(|| meta.get("triggers")),
    );
    if trigger_keywords.is_empty() {
        trigger_keywords = infer_trigger_keywords(&skill_id, &description);
    }

    let mut required_tools = extract_string_array_from_value(
        meta.get("required_tools")
            .or_else(|| meta.get("requiredTools"))
            .or_else(|| meta.get("allowedTools"))
            .or_else(|| meta.get("authorized_tools"))
            .or_else(|| meta.get("tools")),
    );
    if required_tools.is_empty() {
        required_tools = if !md_tools.is_empty() {
            md_tools
        } else {
            default_required_tools()
        };
    }

    let mut invariants = extract_string_array_from_value(
        meta.get("invariants").or_else(|| meta.get("rules")),
    );
    if invariants.is_empty() {
        invariants = md_invariants;
    }

    let mut hooks = extract_hooks_from_value(meta.get("hooks"));
    if hooks.is_empty() {
        hooks = md_hooks;
    }

    let instructions_markdown = if normalized_body.is_empty() {
        meta.get("instructions_markdown")
            .or_else(|| meta.get("instructionsMarkdown"))
            .or_else(|| meta.get("instructions"))
            .or_else(|| meta.get("prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or(&description)
            .trim()
            .to_string()
    } else {
        normalized_body
    };

    let content_sha256 = compute_sha256_hex(&instructions_markdown);

    AlexandriaCapsuleJson {
        schema_version: CANONICAL_CAPSULE_SCHEMA_VERSION.to_string(),
        skill_id,
        domain,
        profile_compatibility,
        description,
        trigger_keywords,
        required_tools,
        invariants,
        hooks,
        instructions_markdown,
        content_sha256,
    }
}

pub fn parse_raw_skill_or_hook(
    file_path: &Path,
    raw_content: &str,
) -> Result<AlexandriaCapsuleJson, String> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("md")
        .to_ascii_lowercase();

    match ext.as_str() {
        "md" | "markdown" => {
            let (frontmatter_opt, body) = split_frontmatter_and_body(raw_content);
            let meta_val: Value = if let Some(yaml_str) = frontmatter_opt {
                serde_yml::from_str(yaml_str).map_err(|e| {
                    format!(
                        "Failed to parse YAML frontmatter in {:?}: {}",
                        file_path, e
                    )
                })?
            } else {
                Value::Object(serde_json::Map::new())
            };
            Ok(build_capsule_from_metadata_and_body(
                &meta_val, body, file_path,
            ))
        }
        "yaml" | "yml" => {
            let meta_val: Value = serde_yml::from_str(raw_content).map_err(|e| {
                format!("Failed to parse YAML skill/hook in {:?}: {}", file_path, e)
            })?;
            let body = meta_val
                .get("instructions_markdown")
                .or_else(|| meta_val.get("instructionsMarkdown"))
                .or_else(|| meta_val.get("instructions"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(build_capsule_from_metadata_and_body(
                &meta_val, body, file_path,
            ))
        }
        "json" => {
            let meta_val: Value = serde_json::from_str(raw_content).map_err(|e| {
                format!("Failed to parse JSON skill/hook in {:?}: {}", file_path, e)
            })?;
            let body = meta_val
                .get("instructions_markdown")
                .or_else(|| meta_val.get("instructionsMarkdown"))
                .or_else(|| meta_val.get("instructions"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(build_capsule_from_metadata_and_body(
                &meta_val, body, file_path,
            ))
        }
        other => Err(format!(
            "Unsupported skill or hook file extension '.{}' in {:?}",
            other, file_path
        )),
    }
}

fn collect_files_recursively(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Err(format!("Source directory does not exist: {:?}", dir));
    }
    if dir.is_file() {
        out.push(dir.to_path_buf());
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;
    let mut sorted_paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    sorted_paths.sort();

    for path in sorted_paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if path.is_dir() {
            collect_files_recursively(&path, out)?;
        } else if path.is_file() {
            if name == MANIFEST_FILENAME {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if matches!(ext.as_str(), "md" | "markdown" | "yaml" | "yml" | "json") {
                out.push(path);
            }
        }
    }
    Ok(())
}

pub fn collect_and_refactor_directory(
    source_dir: &Path,
) -> Result<Vec<AlexandriaCapsuleJson>, String> {
    let mut candidate_files = Vec::new();
    collect_files_recursively(source_dir, &mut candidate_files)?;

    let mut capsules = Vec::new();
    for file_path in candidate_files {
        let raw_content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file {:?}: {}", file_path, e))?;
        let capsule = parse_raw_skill_or_hook(&file_path, &raw_content)?;
        capsules.push(capsule);
    }

    capsules.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    Ok(capsules)
}

pub fn export_refactored_json_bundle(
    capsules: &[AlexandriaCapsuleJson],
    output_dir: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "Failed to create capsule output directory {:?}: {}",
            output_dir, e
        )
    })?;

    let mut manifest_entries = Vec::with_capacity(capsules.len());

    for capsule in capsules {
        let file_name = format!(
            "{}.capsule.json",
            sanitize_skill_id_for_filename(&capsule.skill_id)
        );
        let target_path = output_dir.join(&file_name);
        let serialized = serde_json::to_string_pretty(capsule)
            .map_err(|e| format!("Failed to serialize capsule '{}': {}", capsule.skill_id, e))?;
        fs::write(&target_path, serialized)
            .map_err(|e| format!("Failed to write {:?}: {}", target_path, e))?;

        manifest_entries.push(AlexandriaCapsuleManifestEntry {
            skill_id: capsule.skill_id.clone(),
            domain: capsule.domain.clone(),
            file_name,
            content_sha256: capsule.content_sha256.clone(),
        });
    }

    let manifest = AlexandriaCapsuleManifest {
        schema_version: CANONICAL_CAPSULE_SCHEMA_VERSION.to_string(),
        total_capsules: manifest_entries.len(),
        capsules: manifest_entries,
    };

    let manifest_path = output_dir.join(MANIFEST_FILENAME);
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize capsule manifest: {}", e))?;
    fs::write(&manifest_path, manifest_json)
        .map_err(|e| format!("Failed to write {:?}: {}", manifest_path, e))?;

    Ok(manifest_path)
}

pub fn builtin_core_capsules() -> Vec<AlexandriaCapsuleJson> {
    let tdd_md = "Execute strict RED -> GREEN -> REFACTOR cycles using Vitest with `--outputFile`. Never modify files inside `src/` until a unit test in `tests/` fails deterministically by business `AssertionError`.";
    let prisma_md = "Enforce strict lexical closure inside `prisma.$transaction(async (tx) => { ... })` with sub-25ms execution budgets. Never alias or destructure `tx`, never chain `.then()`, and immediately `await` every `tx.<model>.<op>()` call.";
    let ts_md = "Eliminate primitive obsession using nominal Branded Types (`type Brand<K, T> = K & { readonly __brand: T }`). Explicit `any` (`TSAnyKeyword`) and double casts (`as unknown as T`) are strictly forbidden by Gate 0.";
    let sec_md = "Perform Utu stateless cold-context SAST audits across all diffs, prevent secret or credential leaks, and execute untrusted test suites strictly inside `node --permission` OS sandboxes.";

    vec![
        AlexandriaCapsuleJson {
            schema_version: CANONICAL_CAPSULE_SCHEMA_VERSION.to_string(),
            skill_id: "testing:tdd-vitest".to_string(),
            domain: "testing".to_string(),
            profile_compatibility: default_profiles(),
            description: "Strict RED-GREEN-REFACTOR TDD workflow with Vitest --outputFile JSON verification and mandatory prior AssertionError.".to_string(),
            trigger_keywords: vec![
                "testing".to_string(),
                "tdd".to_string(),
                "vitest".to_string(),
                "red".to_string(),
                "green".to_string(),
                "assertion".to_string(),
            ],
            required_tools: vec![
                "prepare_task_context".to_string(),
                "verify_and_gate_changes".to_string(),
            ],
            invariants: vec![
                "REQUIRE_RED_PHASE_ASSERTION_ERROR".to_string(),
                "MIN_DIFF_COVERAGE_90_PERCENT".to_string(),
                "FORBID_SRC_EDIT_WITHOUT_FAILING_TEST".to_string(),
            ],
            hooks: vec![AlexandriaHookSpec {
                event: "pre_execution".to_string(),
                action: "enforce_red_phase".to_string(),
                target_filter: Some("tests/**/*.test.ts".to_string()),
            }],
            instructions_markdown: tdd_md.to_string(),
            content_sha256: compute_sha256_hex(tdd_md),
        },
        AlexandriaCapsuleJson {
            schema_version: CANONICAL_CAPSULE_SCHEMA_VERSION.to_string(),
            skill_id: "database:prisma-acid-closure".to_string(),
            domain: "database".to_string(),
            profile_compatibility: default_profiles(),
            description: "Strict lexical closure in prisma.$transaction under 25ms, forbidding .then(), tx aliasing, and dangling promises.".to_string(),
            trigger_keywords: vec![
                "database".to_string(),
                "prisma".to_string(),
                "acid".to_string(),
                "closure".to_string(),
                "sub25ms".to_string(),
            ],
            required_tools: vec![
                "prepare_task_context".to_string(),
                "verify_and_gate_changes".to_string(),
            ],
            invariants: vec![
                "PRISMA_TX_INLINE_CALLBACK_REQUIRED".to_string(),
                "PRISMA_TX_NO_ALIAS_OR_DESTRUCTURE".to_string(),
                "PRISMA_TX_NO_PROMISE_CHAINING".to_string(),
                "PRISMA_TX_DANGLING_PROMISE_FORBIDDEN".to_string(),
            ],
            hooks: vec![AlexandriaHookSpec {
                event: "pre_gate_0".to_string(),
                action: "scan_ast_invariants".to_string(),
                target_filter: Some("src/**/*.ts".to_string()),
            }],
            instructions_markdown: prisma_md.to_string(),
            content_sha256: compute_sha256_hex(prisma_md),
        },
        AlexandriaCapsuleJson {
            schema_version: CANONICAL_CAPSULE_SCHEMA_VERSION.to_string(),
            skill_id: "typescript:nominal-branded-types".to_string(),
            domain: "backend".to_string(),
            profile_compatibility: default_profiles(),
            description: "Prohibits explicit any and double casting (as unknown as T), enforcing nominal Branded Types across domain models.".to_string(),
            trigger_keywords: vec![
                "typescript".to_string(),
                "nominal".to_string(),
                "branded".to_string(),
                "types".to_string(),
                "explicit".to_string(),
                "any".to_string(),
            ],
            required_tools: vec![
                "prepare_task_context".to_string(),
                "verify_and_gate_changes".to_string(),
            ],
            invariants: vec![
                "NO_EXPLICIT_ANY".to_string(),
                "NO_DOUBLE_CAST_UNKNOWN".to_string(),
                "ENFORCE_NOMINAL_BRANDED_TYPES".to_string(),
            ],
            hooks: vec![AlexandriaHookSpec {
                event: "pre_gate_0".to_string(),
                action: "scan_ast_invariants".to_string(),
                target_filter: Some("src/**/*.{ts,tsx}".to_string()),
            }],
            instructions_markdown: ts_md.to_string(),
            content_sha256: compute_sha256_hex(ts_md),
        },
        AlexandriaCapsuleJson {
            schema_version: CANONICAL_CAPSULE_SCHEMA_VERSION.to_string(),
            skill_id: "security:zero-trust-sast".to_string(),
            domain: "security".to_string(),
            profile_compatibility: default_profiles(),
            description: "Utu stateless cold-audit rules, secret leak prevention, and OS sandboxed execution via node --permission.".to_string(),
            trigger_keywords: vec![
                "security".to_string(),
                "zero".to_string(),
                "trust".to_string(),
                "sast".to_string(),
                "sandbox".to_string(),
                "permission".to_string(),
            ],
            required_tools: vec![
                "verify_and_gate_changes".to_string(),
                "finalize_task_and_log".to_string(),
            ],
            invariants: vec![
                "ZERO_SECRET_LEAKS".to_string(),
                "ENFORCE_NODE_PERMISSION_SANDBOX".to_string(),
                "STATELESS_UTU_COLD_AUDIT".to_string(),
            ],
            hooks: vec![AlexandriaHookSpec {
                event: "post_gate_3".to_string(),
                action: "run_stateless_utu_audit".to_string(),
                target_filter: Some("src/**".to_string()),
            }],
            instructions_markdown: sec_md.to_string(),
            content_sha256: compute_sha256_hex(sec_md),
        },
    ]
}
