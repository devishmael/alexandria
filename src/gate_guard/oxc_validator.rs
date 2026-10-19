use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_ast_visit::walk;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

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
