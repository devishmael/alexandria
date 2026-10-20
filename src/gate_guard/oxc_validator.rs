use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstViolation {
    pub rule: &'static str,
    pub message: String,
    pub byte_offset: u32,
}

pub struct GateZeroAstVisitor {
    pub violations: Vec<AstViolation>,
    active_tx_params: Vec<String>,
}

impl Default for GateZeroAstVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl GateZeroAstVisitor {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            active_tx_params: Vec::new(),
        }
    }

    fn current_tx_param(&self) -> Option<String> {
        self.active_tx_params.last().cloned()
    }

    fn is_strict_tx_operation(callee: &Expression<'_>, tx_name: &str) -> bool {
        match callee {
            Expression::StaticMemberExpression(outer) => {
                let prop = outer.property.name.as_str();
                if matches!(prop, "then" | "catch" | "finally") {
                    return false;
                }
                match &outer.object {
                    Expression::Identifier(ident) => ident.name.as_str() == tx_name,
                    Expression::StaticMemberExpression(inner) => {
                        if let Expression::Identifier(ident) = &inner.object {
                            ident.name.as_str() == tx_name
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for GateZeroAstVisitor {
    fn visit_ts_any_keyword(&mut self, it: &TSAnyKeyword) {
        self.violations.push(AstViolation {
            rule: "NO_EXPLICIT_ANY",
            message: "Explicit 'any' type is forbidden. Use nominal types or Branded Types.".into(),
            byte_offset: it.span.start,
        });
    }

    fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
        if let Expression::TSAsExpression(inner_as) = &it.expression {
            if matches!(inner_as.type_annotation, TSType::TSUnknownKeyword(_)) {
                self.violations.push(AstViolation {
                    rule: "NO_DOUBLE_CAST_UNKNOWN",
                    message: "Double type assertion 'as unknown as T' is forbidden to prevent type-system evasion.".into(),
                    byte_offset: it.span.start,
                });
            }
        }
        walk::walk_ts_as_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            if let Some(init_expr) = &it.init {
                match init_expr {
                    Expression::Identifier(ident) if ident.name.as_str() == tx_name => {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_NO_ALIASING",
                            message: format!(
                                "Reassigning or aliasing the transactional client '{}' is forbidden.",
                                tx_name
                            ),
                            byte_offset: it.span.start,
                        });
                    }
                    Expression::CallExpression(call)
                        if Self::is_strict_tx_operation(&call.callee, &tx_name) =>
                    {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT",
                            message: format!(
                                "Storing a promise from '{}' in a variable without immediate 'await' is forbidden.",
                                tx_name
                            ),
                            byte_offset: it.span.start,
                        });
                    }
                    _ => {}
                }
            }
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            match &it.right {
                Expression::Identifier(ident) if ident.name.as_str() == tx_name => {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_NO_ASSIGNMENT_ESCAPE",
                        message: format!(
                            "Assigning the transactional client '{}' to external variables is forbidden.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
                Expression::CallExpression(call)
                    if Self::is_strict_tx_operation(&call.callee, &tx_name) =>
                {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_UNWAITED_PROMISE_ASSIGNMENT",
                        message: format!(
                            "Assigning a promise from '{}' to a variable without immediate 'await' is forbidden.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
                _ => {}
            }
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            if let Expression::CallExpression(call) = &it.expression {
                if Self::is_strict_tx_operation(&call.callee, &tx_name) {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_DANGLING_PROMISE",
                        message: format!(
                            "Unawaited operation on '{}' detected; this will cause premature transaction closure.",
                            tx_name
                        ),
                        byte_offset: it.span.start,
                    });
                }
            }
        }
        walk::walk_expression_statement(self, it);
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            let valid = match &it.argument {
                Expression::CallExpression(call) => {
                    Self::is_strict_tx_operation(&call.callee, &tx_name)
                }
                Expression::TaggedTemplateExpression(tagged) => {
                    Self::is_strict_tx_operation(&tagged.tag, &tx_name)
                }
                _ => false,
            };

            if !valid {
                self.violations.push(AstViolation {
                    rule: "PRISMA_TX_STRICT_LEXICAL_AWAIT",
                    message: format!(
                        "Inside $transaction only direct await calls 'await {}.<model>.<op>()' are allowed. External I/O, .then(), or Promise.all are forbidden.",
                        tx_name
                    ),
                    byte_offset: it.span.start,
                });
            }
        }
        walk::walk_await_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(tx_name) = self.current_tx_param() {
            for arg in &it.arguments {
                if let Argument::Identifier(ident) = arg {
                    if ident.name.as_str() == tx_name {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_NO_ESCAPE",
                            message: format!(
                                "Passing the transactional client '{}' to helper functions is forbidden.",
                                tx_name
                            ),
                            byte_offset: ident.span.start,
                        });
                    }
                }
            }

            if let Expression::Identifier(ident) = &it.callee {
                if matches!(ident.name.as_str(), "fetch" | "setTimeout" | "setInterval") {
                    self.violations.push(AstViolation {
                        rule: "PRISMA_TX_NO_UNWAITED_IO",
                        message: format!(
                            "Invoking '{}' inside a database transaction is forbidden.",
                            ident.name.as_str()
                        ),
                        byte_offset: it.span.start,
                    });
                }
            }
        }

        let is_prisma_tx_call = match &it.callee {
            Expression::StaticMemberExpression(member) => {
                member.property.name.as_str() == "$transaction"
            }
            _ => false,
        };

        if is_prisma_tx_call {
            if let Some(first_arg) = it.arguments.first() {
                match first_arg {
                    Argument::ArrayExpression(_) => {
                        walk::walk_call_expression(self, it);
                        return;
                    }
                    Argument::ArrowFunctionExpression(arrow) => {
                        if let Some(first_param) = arrow.params.items.first() {
                            match &first_param.pattern.kind {
                                BindingPatternKind::BindingIdentifier(ident) => {
                                    let param_name = ident.name.as_str().to_string();
                                    self.active_tx_params.push(param_name);
                                    walk::walk_arrow_function_expression(self, arrow);
                                    self.active_tx_params.pop();
                                    return;
                                }
                                _ => {
                                    self.violations.push(AstViolation {
                                        rule: "PRISMA_TX_NO_PARAM_DESTRUCTURING",
                                        message: "The $transaction parameter must be a simple identifier (e.g. 'async (tx) => ...'), without destructuring.".into(),
                                        byte_offset: first_param.span.start,
                                    });
                                    return;
                                }
                            }
                        }
                    }
                    _ => {
                        self.violations.push(AstViolation {
                            rule: "PRISMA_TX_REQUIRE_INLINE_CALLBACK",
                            message: "The first argument to $transaction must be an inline ArrowFunction or a static Array.".into(),
                            byte_offset: first_arg.span().start,
                        });
                        return;
                    }
                }
            }
        }

        walk::walk_call_expression(self, it);
    }
}

pub fn validate_typescript_source(
    source_text: &str,
    is_tsx: bool,
) -> Result<Vec<AstViolation>, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_jsx(is_tsx);
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    if !ret.errors.is_empty() {
        return Err(format!("SyntaxError during Oxc parsing: {:?}", ret.errors[0]));
    }

    let mut visitor = GateZeroAstVisitor::new();
    visitor.visit_program(&ret.program);
    Ok(visitor.violations)
}

pub fn extract_public_export_signatures(
    source_text: &str,
    is_tsx: bool,
) -> Result<Vec<String>, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_jsx(is_tsx);
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    if !ret.errors.is_empty() {
        return Err(format!("SyntaxError during Oxc parsing: {:?}", ret.errors[0]));
    }

    let mut signatures: Vec<String> = Vec::new();

    for stmt in &ret.program.body {
        match stmt {
            Statement::ExportNamedDeclaration(export_named) => {
                if let Some(decl) = &export_named.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(func) => {
                            if let Some(id) = &func.id {
                                signatures.push(format!(
                                    "export function {}({})",
                                    id.name.as_str(),
                                    func.params.items.len()
                                ));
                            }
                        }
                        Declaration::TSInterfaceDeclaration(iface) => {
                            signatures.push(format!(
                                "export interface {}",
                                iface.id.name.as_str()
                            ));
                        }
                        Declaration::TSTypeAliasDeclaration(type_alias) => {
                            signatures
                                .push(format!("export type {}", type_alias.id.name.as_str()));
                        }
                        Declaration::ClassDeclaration(cls) => {
                            if let Some(id) = &cls.id {
                                signatures.push(format!("export class {}", id.name.as_str()));
                            }
                        }
                        Declaration::TSEnumDeclaration(ts_enum) => {
                            signatures.push(format!("export enum {}", ts_enum.id.name.as_str()));
                        }
                        Declaration::VariableDeclaration(var_decl) => {
                            for declarator in &var_decl.declarations {
                                if let BindingPatternKind::BindingIdentifier(ident) =
                                    &declarator.id.kind
                                {
                                    signatures.push(format!(
                                        "export const {}",
                                        ident.name.as_str()
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                for spec in &export_named.specifiers {
                    signatures.push(format!("export specifier {}", spec.exported.name()));
                }
            }
            Statement::ExportDefaultDeclaration(export_default) => {
                match &export_default.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                        let name = func
                            .id
                            .as_ref()
                            .map(|id| id.name.as_str())
                            .unwrap_or("default");
                        signatures.push(format!(
                            "export default function {}({})",
                            name,
                            func.params.items.len()
                        ));
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(cls) => {
                        let name = cls
                            .id
                            .as_ref()
                            .map(|id| id.name.as_str())
                            .unwrap_or("default");
                        signatures.push(format!("export default class {}", name));
                    }
                    _ => {
                        signatures.push("export default expression".to_string());
                    }
                }
            }
            _ => {}
        }
    }

    signatures.sort();
    signatures.dedup();
    Ok(signatures)
}

pub fn detect_public_export_signature_change(
    baseline_source: &str,
    modified_source: &str,
    is_tsx: bool,
) -> Result<bool, String> {
    let base_sigs = extract_public_export_signatures(baseline_source, is_tsx)?;
    let mod_sigs = extract_public_export_signatures(modified_source, is_tsx)?;
    Ok(base_sigs != mod_sigs)
}

pub fn scan_gate2_sast_and_secrets(source_text: &str) -> Vec<AstViolation> {
    let mut violations = Vec::new();

    let secret_markers: [(&str, &str); 4] = [
        ("sk-", "OpenAI/Stripe-style secret key prefix 'sk-'"),
        ("ghp_", "GitHub Personal Access Token prefix 'ghp_'"),
        ("AKIA", "AWS Access Key ID prefix 'AKIA'"),
        (
            "-----BEGIN PRIVATE KEY-----",
            "PEM-encoded private key block",
        ),
    ];

    for (pattern, label) in secret_markers {
        if let Some(idx) = source_text.find(pattern) {
            violations.push(AstViolation {
                rule: "GATE2_HARDCODED_SECRET",
                message: format!(
                    "Gate 2 Secret Leak Guard: detected hardcoded credential ({}) in source.",
                    label
                ),
                byte_offset: idx as u32,
            });
        }
    }

    let unsafe_execution_markers: [(&str, &str); 3] = [
        ("eval(", "dynamic code evaluation via 'eval(...)'"),
        (
            "new Function(",
            "dynamic function constructor 'new Function(...)'",
        ),
        (
            "dangerouslySetInnerHTML",
            "unsanitized DOM injection via 'dangerouslySetInnerHTML'",
        ),
    ];

    for (pattern, label) in unsafe_execution_markers {
        if let Some(idx) = source_text.find(pattern) {
            violations.push(AstViolation {
                rule: "GATE2_SAST_UNSAFE_EXECUTION",
                message: format!(
                    "Gate 2 SAST Scanner: forbidden unsafe execution vector ({}).",
                    label
                ),
                byte_offset: idx as u32,
            });
        }
    }

    violations
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrunedAstContext {
    pub target_symbol: String,
    pub retained_symbols: Vec<String>,
    pub pruned_symbols: Vec<String>,
    pub pruned_source: String,
    pub original_bytes: usize,
    pub pruned_bytes: usize,
    pub estimated_tokens_saved: usize,
}

struct CallIdentifierCollector {
    called_identifiers: Vec<String>,
}

impl CallIdentifierCollector {
    fn new() -> Self {
        Self {
            called_identifiers: Vec::new(),
        }
    }
}

impl<'a> Visit<'a> for CallIdentifierCollector {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(ident) = &it.callee {
            let name = ident.name.as_str().to_string();
            if !self.called_identifiers.contains(&name) {
                self.called_identifiers.push(name);
            }
        }
        walk::walk_call_expression(self, it);
    }
}

pub fn prune_typescript_context_for_symbol(
    source_text: &str,
    target_symbol: &str,
    is_tsx: bool,
) -> Result<PrunedAstContext, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_jsx(is_tsx);
    let ret = Parser::new(&allocator, source_text, source_type).parse();

    if !ret.errors.is_empty() {
        return Err(format!("SyntaxError during Oxc parsing: {:?}", ret.errors[0]));
    }

    struct FunctionEntry {
        name: String,
        body_start: usize,
        body_end: usize,
        called_names: Vec<String>,
    }

    let mut functions: Vec<FunctionEntry> = Vec::new();
    let mut type_and_interface_symbols: Vec<String> = Vec::new();

    for stmt in &ret.program.body {
        match stmt {
            Statement::FunctionDeclaration(func) => {
                if let (Some(id), Some(body)) = (&func.id, &func.body) {
                    let mut collector = CallIdentifierCollector::new();
                    collector.visit_function_body(body);
                    functions.push(FunctionEntry {
                        name: id.name.as_str().to_string(),
                        body_start: body.span.start as usize,
                        body_end: body.span.end as usize,
                        called_names: collector.called_identifiers,
                    });
                }
            }
            Statement::TSInterfaceDeclaration(iface) => {
                type_and_interface_symbols.push(iface.id.name.as_str().to_string());
            }
            Statement::TSTypeAliasDeclaration(ta) => {
                type_and_interface_symbols.push(ta.id.name.as_str().to_string());
            }
            Statement::ExportNamedDeclaration(exp) => {
                if let Some(decl) = &exp.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(func) => {
                            if let (Some(id), Some(body)) = (&func.id, &func.body) {
                                let mut collector = CallIdentifierCollector::new();
                                collector.visit_function_body(body);
                                functions.push(FunctionEntry {
                                    name: id.name.as_str().to_string(),
                                    body_start: body.span.start as usize,
                                    body_end: body.span.end as usize,
                                    called_names: collector.called_identifiers,
                                });
                            }
                        }
                        Declaration::TSInterfaceDeclaration(iface) => {
                            type_and_interface_symbols.push(iface.id.name.as_str().to_string());
                        }
                        Declaration::TSTypeAliasDeclaration(ta) => {
                            type_and_interface_symbols.push(ta.id.name.as_str().to_string());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let mut retained_fn_names: Vec<String> = vec![target_symbol.to_string()];
    if let Some(target_entry) = functions.iter().find(|f| f.name == target_symbol) {
        for called in &target_entry.called_names {
            if functions.iter().any(|f| &f.name == called) && !retained_fn_names.contains(called) {
                retained_fn_names.push(called.clone());
            }
        }
    }

    let mut retained_symbols = type_and_interface_symbols;
    let mut pruned_symbols = Vec::new();
    let mut spans_to_prune: Vec<(usize, usize)> = Vec::new();

    for f in &functions {
        if retained_fn_names.contains(&f.name) {
            if !retained_symbols.contains(&f.name) {
                retained_symbols.push(f.name.clone());
            }
        } else {
            pruned_symbols.push(f.name.clone());
            spans_to_prune.push((f.body_start, f.body_end));
        }
    }

    spans_to_prune.sort_by_key(|&(start, _)| start);

    let mut pruned_source = String::with_capacity(source_text.len());
    let mut cursor = 0usize;
    for (start, end) in spans_to_prune {
        if start >= cursor && end <= source_text.len() && start <= end {
            pruned_source.push_str(&source_text[cursor..start]);
            pruned_source.push_str("{ /* [Alexandria AST Pruned] */ }");
            cursor = end;
        }
    }
    if cursor < source_text.len() {
        pruned_source.push_str(&source_text[cursor..]);
    }

    let original_bytes = source_text.len();
    let pruned_bytes = pruned_source.len();
    let estimated_tokens_saved = original_bytes.saturating_sub(pruned_bytes) / 4;

    Ok(PrunedAstContext {
        target_symbol: target_symbol.to_string(),
        retained_symbols,
        pruned_symbols,
        pruned_source,
        original_bytes,
        pruned_bytes,
        estimated_tokens_saved,
    })
}
