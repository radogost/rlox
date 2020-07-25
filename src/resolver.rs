use crate::error::{LoxError, Result};
use crate::statement::{Expr, ExprId, Stmt};

use std::collections::HashMap;

pub type Depth = u64;

struct Resolver<'a> {
    scopes: Vec<HashMap<&'a str, bool>>,
    expr_id_to_depth: HashMap<ExprId, Depth>,
}

impl<'a> Resolver<'a> {
    fn new() -> Self {
        Self {
            scopes: Vec::new(),
            expr_id_to_depth: HashMap::new(),
        }
    }

    fn resolve(&mut self, statements: &'a [Stmt]) -> Result<HashMap<ExprId, Depth>> {
        self.resolve_statements(statements)?;
        Ok(std::mem::take(&mut self.expr_id_to_depth))
    }

    fn resolve_statements(&mut self, stmts: &'a [Stmt]) -> Result<()> {
        for stmt in stmts {
            self.resolve_statement(stmt)?;
        }
        Ok(())
    }

    fn resolve_statement(&mut self, stmt: &'a Stmt) -> Result<()> {
        match stmt {
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve_statements(statements.as_ref())?;
                self.end_scope();
            }
            Stmt::Var { name, initializer } => {
                self.declare(name);
                self.define(name);
                if let Some(initializer) = initializer {
                    self.resolve_expression(initializer)?;
                }
            }
            Stmt::Function {
                name,
                parameters,
                body,
            } => {
                self.declare(name);
                self.define(name);
                self.begin_scope();
                for param in parameters.as_ref() {
                    self.declare(param);
                    self.define(param);
                }
                self.resolve_statements(body)?;
                self.end_scope();
            }
            Stmt::Expression { expression } => {
                self.resolve_expression(expression)?;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expression(condition)?;
                self.resolve_statement(then_branch.as_ref())?;
                if let Some(stmt) = else_branch {
                    self.resolve_statement(stmt)?;
                }
            }
            Stmt::Print { expression } => self.resolve_expression(expression)?,
            Stmt::Return { value } => {
                if let Some(value) = value {
                    self.resolve_expression(value)?;
                }
            }
            Stmt::While { condition, body } => {
                self.resolve_expression(condition)?;
                self.resolve_statement(body)?;
            }
        };
        Ok(())
    }

    fn resolve_expression(&mut self, expr: &'a Expr) -> Result<()> {
        match expr {
            Expr::Variable { id, name } => {
                if let Some(scope) = self.scopes.last() {
                    if scope.get::<str>(name) == Some(&false) {
                        return Err(LoxError::ResolverError(
                            "Cannot read local variable in ints own initializer",
                        ));
                    }
                    self.resolve_local(*id, name);
                }
            }
            Expr::Assign { id, value, name } => {
                self.resolve_expression(value)?;
                self.resolve_local(*id, name);
            }
            Expr::Binary {
                left,
                token_type: _,
                right,
            } => {
                self.resolve_expression(left)?;
                self.resolve_expression(right)?;
            }
            Expr::Call { callee, arguments } => {
                self.resolve_expression(callee)?;
                for arg in arguments.as_ref() {
                    self.resolve_expression(arg)?;
                }
            }
            Expr::Grouping { expression } => {
                self.resolve_expression(expression)?;
            }
            Expr::Logical {
                left,
                operator: _,
                right,
            } => {
                self.resolve_expression(left)?;
                self.resolve_expression(right)?;
            }
            Expr::Unary {
                token_type: _,
                right,
            } => {
                self.resolve_expression(right)?;
            }
            Expr::Nil | Expr::Boolean(_) | Expr::Number(_) | Expr::String(_) => {}
        };
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &'a str) {
        self.scopes
            .last_mut()
            .map(|scope| scope.insert(name, false));
    }

    fn define(&mut self, name: &'a str) {
        self.scopes.last_mut().map(|scope| scope.insert(name, true));
    }

    fn resolve_local(&mut self, expr_id: ExprId, name: &'a str) {
        self.scopes
            .iter_mut()
            .rev()
            .enumerate()
            .find(|(_, scope)| scope.contains_key(name))
            .map(|(depth, _)| (expr_id, depth as u64))
            .map(|(expr_id, depth)| self.expr_id_to_depth.insert(expr_id, depth));
    }
}

pub fn resolve(statements: &[Stmt]) -> Result<HashMap<ExprId, Depth>> {
    let mut resolver = Resolver::new();
    resolver.resolve(statements)
}