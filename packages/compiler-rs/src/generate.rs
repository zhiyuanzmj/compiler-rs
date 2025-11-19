pub mod block;
pub mod component;
pub mod directive;
pub mod dom;
pub mod event;
pub mod expression;
pub mod html;
pub mod operation;
pub mod prop;
pub mod slot;
pub mod template;
pub mod template_ref;
pub mod text;
pub mod utils;
pub mod v_for;
pub mod v_if;
pub mod v_model;
pub mod v_show;

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  mem,
};

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{
  AstBuilder, NONE,
  ast::{Expression, FormalParameterKind, Program, Statement, VariableDeclarationKind},
};
use oxc_span::{SPAN, SourceType};

use crate::{
  compile::Template,
  generate::block::gen_block_content,
  ir::index::{BlockIRNode, RootIRNode},
  transform::TransformOptions,
};

pub struct CodegenContext<'a> {
  pub options: &'a TransformOptions<'a>,
  pub identifiers: RefCell<HashMap<String, Vec<Expression<'a>>>>,
  pub ir: &'a RootIRNode<'a>,
  pub block: RefCell<BlockIRNode<'a>>,
  pub scope_level: RefCell<i32>,
  pub ast: AstBuilder<'a>,
}

impl<'a> CodegenContext<'a> {
  pub fn new(
    allocator: &'a Allocator,
    ir: &'a RootIRNode<'a>,
    block: BlockIRNode<'a>,
    options: &'a TransformOptions<'a>,
  ) -> CodegenContext<'a> {
    let ast = AstBuilder::new(allocator);
    CodegenContext {
      options,
      identifiers: RefCell::new(HashMap::new()),
      block: RefCell::new(block),
      scope_level: RefCell::new(0),
      ir,
      ast,
    }
  }

  pub fn helper(&self, name: &str) -> String {
    self.options.helpers.borrow_mut().insert(name.to_string());
    format!("_{name}")
  }

  pub fn with_id(
    &self,
    _fn: impl FnOnce() -> Expression<'a>,
    id_map: &HashMap<String, Option<Expression<'a>>>,
  ) -> Expression<'a> {
    let ids = id_map.keys();
    for id in ids {
      let mut identifiers = self.identifiers.borrow_mut();
      if identifiers.get(id).is_none() {
        identifiers.insert(id.clone(), vec![]);
      }
      identifiers.get_mut(id).unwrap().insert(
        0,
        if let Some(value) = id_map.get(id) {
          if let Some(value) = value {
            value.clone_in(self.ast.allocator)
          } else {
            self.ast.expression_identifier(SPAN, self.ast.atom(&id))
          }
        } else {
          self.ast.expression_identifier(SPAN, self.ast.atom(&id))
        },
      );
    }

    let ret = _fn();

    for id in id_map.keys() {
      if let Some(ids) = self.identifiers.borrow_mut().get_mut(id) {
        ids.clear();
      }
    }

    ret
  }

  pub fn enter_block(
    &self,
    block: BlockIRNode<'a>,
    context_block: &mut BlockIRNode<'a>,
  ) -> impl FnOnce() {
    let parent = mem::take(context_block);
    *context_block = block;
    || *context_block = parent
  }

  pub fn enter_scope(&self) -> (i32, impl FnOnce()) {
    let mut scope_level = self.scope_level.borrow_mut();
    let current = *scope_level;
    *scope_level += 1;
    (current, || *self.scope_level.borrow_mut() -= 1)
  }
}

pub struct VaporCodegenResult<'a> {
  pub helpers: HashSet<String>,
  pub templates: Vec<Template>,
  pub delegates: HashSet<String>,
  pub program: Program<'a>,
}

// IR -> JS codegen
pub fn generate<'a>(context: &'a CodegenContext<'a>) -> Program<'a> {
  let ir = &context.ir;
  let source = ir.source;
  let ast = &context.ast;
  let mut statements = ast.vec();

  if ir.has_template_ref {
    statements.push(Statement::VariableDeclaration(
      ast.alloc_variable_declaration(
        SPAN,
        VariableDeclarationKind::Const,
        ast.vec1(ast.variable_declarator(
          SPAN,
          VariableDeclarationKind::Const,
          ast.binding_pattern(
            ast.binding_pattern_kind_binding_identifier(SPAN, ast.atom("_setTemplateRef")),
            NONE,
            false,
          ),
          Some(ast.expression_call(
            SPAN,
            ast.expression_identifier(SPAN, ast.atom(&context.helper("createTemplateRefSetter"))),
            NONE,
            ast.vec(),
            false,
          )),
          false,
        )),
        false,
      ),
    ));
  }
  let context_block = &mut *context.block.borrow_mut() as *mut BlockIRNode;
  statements.extend(gen_block_content(
    None,
    &context,
    unsafe { &mut *context_block },
    true,
    None,
  ));

  if !context.options.delegates.borrow().is_empty() {
    context.helper("delegateEvents");
  }
  if !&context.options.templates.borrow().is_empty() {
    context.helper("template");
  }

  ast.program(
    SPAN,
    SourceType::tsx(),
    source,
    ast.vec(),
    None,
    ast.vec(),
    ast.vec1(ast.statement_expression(
      SPAN,
      ast.expression_call(
        SPAN,
        ast.expression_parenthesized(
          SPAN,
          ast.expression_arrow_function(
            SPAN,
            false,
            false,
            NONE,
            ast.formal_parameters(
              SPAN,
              FormalParameterKind::ArrowFormalParameters,
              ast.vec(),
              NONE,
            ),
            NONE,
            ast.function_body(SPAN, ast.vec(), statements),
          ),
        ),
        NONE,
        ast.vec(),
        false,
      ),
    )),
  )
}
