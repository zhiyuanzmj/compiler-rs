use oxc_ast::ast::{
  BindingIdentifier, BindingPattern, BindingPatternKind, ForInStatement, ForOfStatement,
  ForStatement, ForStatementInit, ForStatementLeft, Statement, VariableDeclarationKind,
};
use std::collections::{HashMap, HashSet};

use napi::bindgen_prelude::Either3;
use oxc_ast::{AstKind, ast::IdentifierReference};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

use crate::utils::check::is_referenced_identifier;

/**
 * Modified from https://github.com/vuejs/core/blob/main/packages/compiler-core/src/babelUtils.ts
 * To support browser environments and JSX.
 *
 * https://github.com/vuejs/core/blob/main/LICENSE
 *
 * Return value indicates whether the AST walked can be a constant
 */
pub struct WalkIdentifiers<'a> {
  known_ids: HashMap<String, u32>,
  parent_stack: Vec<AstKind<'a>>,
  include_all: bool,
  on_identifier:
    Box<dyn FnMut(&IdentifierReference, &Option<&AstKind>, &Vec<AstKind>, bool, bool) + 'a>,
  scope_ids_map: HashMap<Span, HashSet<String>>,
}

impl<'a> WalkIdentifiers<'a> {
  pub fn new(
    on_identifier: Box<
      dyn FnMut(&IdentifierReference, &Option<&AstKind>, &Vec<AstKind>, bool, bool) + 'a,
    >,
    include_all: bool,
  ) -> Self {
    Self {
      on_identifier,
      include_all,
      parent_stack: vec![],
      known_ids: HashMap::new(),
      scope_ids_map: HashMap::new(),
    }
  }
}

impl<'a> Visit<'a> for WalkIdentifiers<'a> {
  fn enter_node(&mut self, node: AstKind<'a>) {
    let parent_stack = &mut self.parent_stack;
    let parent = parent_stack.last();
    if let Some(parent) = parent
      && parent.is_type()
    {
      return;
    }

    let known_ids = &mut self.known_ids;
    let scope_ids_map = &mut self.scope_ids_map;

    if let AstKind::IdentifierReference(node) = node {
      let is_local = known_ids.contains_key(node.name.as_str());
      let is_refed = is_referenced_identifier(node, &parent, &parent_stack);
      if self.include_all || (is_refed && !is_local) {
        self.on_identifier.as_mut()(node, &parent, &parent_stack, is_refed, is_local);
      }
    } else if node.is_function_like() {
      if let Some(scope_ids) = scope_ids_map.get(&node.span()) {
        for id in scope_ids {
          mark_known_ids(id.clone(), known_ids);
        }
      } else {
        // walk function expressions and add its arguments to known identifiers
        // so that we don't prefix them
        walk_function_params(&node, |id| {
          mark_scope_identifier(&node, id, known_ids, scope_ids_map);
        });
      }
    } else if let AstKind::FunctionBody(body) = node {
      if let Some(scope_ids) = scope_ids_map.get(&node.span()) {
        for id in scope_ids {
          mark_known_ids(id.clone(), known_ids);
        }
      } else {
        walk_block_declarations(&body.statements, |id| {
          mark_scope_identifier(&node, id, known_ids, scope_ids_map);
        });
      }
    } else if let AstKind::BlockStatement(block) = node {
      if let Some(scope_ids) = scope_ids_map.get(&node.span()) {
        for id in scope_ids {
          mark_known_ids(id.clone(), known_ids);
        }
      } else {
        // #3445 record block-level local variables
        walk_block_declarations(&block.body, |id| {
          mark_scope_identifier(&node, id, known_ids, scope_ids_map);
        });
      }
    } else if let AstKind::CatchClause(catch) = node
      && let Some(param) = &catch.param
    {
      for id in extract_identifiers(&param.pattern, vec![]) {
        mark_scope_identifier(&node, id, known_ids, scope_ids_map);
      }
    } else if let AstKind::ForStatement(stmt) = node {
      walk_for_statement(Either3::A(&stmt), true, &mut |id| {
        mark_scope_identifier(&node, id, known_ids, scope_ids_map);
      })
    } else if let AstKind::ForInStatement(stmt) = node {
      walk_for_statement(Either3::B(&stmt), true, &mut |id| {
        mark_scope_identifier(&node, id, known_ids, scope_ids_map);
      })
    } else if let AstKind::ForOfStatement(stmt) = node {
      walk_for_statement(Either3::C(&stmt), true, &mut |id| {
        mark_scope_identifier(&node, id, known_ids, scope_ids_map);
      })
    }
    parent_stack.push(node);
  }
  fn leave_node(&mut self, node: AstKind<'a>) {
    let parent = self.parent_stack.pop();
    let known_ids = &mut self.known_ids;
    if parent.is_some()
      && let Some(scope_ids) = self.scope_ids_map.get(&node.span())
    {
      for id in scope_ids {
        if let Some(size) = known_ids.get(id) {
          known_ids.insert(id.clone(), size - 1);
          if known_ids[id] == 0 {
            known_ids.remove(id);
          }
        }
      }
    }
  }
}

pub fn mark_known_ids(name: String, known_ids: &mut HashMap<String, u32>) {
  if let Some(ids) = known_ids.get(&name) {
    known_ids.insert(name, ids + 1);
  } else {
    known_ids.insert(name, 1);
  }
}

pub fn mark_scope_identifier(
  node: &AstKind,
  child: &BindingIdentifier,
  known_ids: &mut HashMap<String, u32>,
  scope_ids_map: &mut HashMap<Span, HashSet<String>>,
) {
  let name = child.name.to_string();
  if let Some(scope_ids) = scope_ids_map.get_mut(&node.span()) {
    if scope_ids.contains(&name) {
      return;
    } else {
      scope_ids.insert(name.clone());
    }
  } else {
    scope_ids_map.insert(node.span(), HashSet::from([name.clone()]));
  }
  mark_known_ids(name, known_ids);
}

pub fn walk_function_params<'a>(
  node: &'a AstKind,
  mut on_ident: impl FnMut(&'a BindingIdentifier) + 'a,
) {
  let params = match node {
    AstKind::Function(node) => &node.params.items,
    AstKind::ArrowFunctionExpression(node) => &node.params.items,
    _ => panic!(""),
  };
  for p in params {
    for id in extract_identifiers(&p.pattern, Vec::new()) {
      on_ident(id)
    }
  }
}

pub fn extract_identifiers<'a>(
  node: &'a BindingPattern<'a>,
  mut identifiers: Vec<&'a BindingIdentifier<'a>>,
) -> Vec<&'a BindingIdentifier<'a>> {
  match &node.kind {
    BindingPatternKind::BindingIdentifier(node) => identifiers.push(node.as_ref()),
    BindingPatternKind::ObjectPattern(node) => {
      if let Some(rest) = &node.rest {
        identifiers = extract_identifiers(&rest.argument, identifiers);
      } else {
        for prop in &node.properties {
          identifiers = extract_identifiers(&prop.value, identifiers)
        }
      }
    }
    BindingPatternKind::ArrayPattern(node) => {
      for element in &node.elements {
        if let Some(element) = element {
          identifiers = extract_identifiers(element, identifiers);
        }
      }
    }
    BindingPatternKind::AssignmentPattern(node) => {
      identifiers = extract_identifiers(&node.left, identifiers);
    }
  }
  identifiers
}

pub fn walk_block_declarations<'a>(
  body: &'a oxc_allocator::Vec<Statement>,
  mut on_ident: impl FnMut(&'a BindingIdentifier) + 'a,
) {
  for stmt in body {
    if let Statement::VariableDeclaration(stmt) = stmt {
      if stmt.declare {
        continue;
      }
      for decl in &stmt.declarations {
        for id in extract_identifiers(&decl.id, Vec::new()) {
          on_ident(id)
        }
      }
    } else if let Statement::FunctionDeclaration(stmt) = stmt {
      if stmt.declare {
        continue;
      }
      if let Some(id) = &stmt.id {
        on_ident(id);
      }
    } else if let Statement::ClassDeclaration(stmt) = stmt {
      if stmt.declare {
        continue;
      }
      if let Some(id) = &stmt.id {
        on_ident(id);
      }
    } else if let Statement::ForStatement(stmt) = stmt {
      walk_for_statement(Either3::A(&stmt), true, &mut on_ident);
    } else if let Statement::ForInStatement(stmt) = stmt {
      walk_for_statement(Either3::B(&stmt), true, &mut on_ident);
    } else if let Statement::ForOfStatement(stmt) = stmt {
      walk_for_statement(Either3::C(&stmt), true, &mut on_ident);
    }
  }
}

pub fn walk_for_statement<'a>(
  stmt: Either3<&'a ForStatement, &'a ForInStatement, &'a ForOfStatement>,
  is_var: bool,
  on_ident: &mut impl FnMut(&'a BindingIdentifier),
) {
  let variable = if let Either3::A(stmt) = stmt
    && let Some(ForStatementInit::VariableDeclaration(stmt)) = &stmt.init
  {
    Some(stmt.as_ref())
  } else if let Either3::B(stmt) = stmt
    && let ForStatementLeft::VariableDeclaration(stmt) = &stmt.left
  {
    Some(stmt.as_ref())
  } else if let Either3::C(stmt) = stmt
    && let ForStatementLeft::VariableDeclaration(stmt) = &stmt.left
  {
    Some(stmt.as_ref())
  } else {
    None
  };
  if let Some(variable) = variable
    && if let VariableDeclarationKind::Var = variable.kind {
      is_var
    } else {
      !is_var
    }
  {
    for decl in &variable.declarations {
      for id in extract_identifiers(&decl.id, Vec::new()) {
        on_ident(id)
      }
    }
  }
}
