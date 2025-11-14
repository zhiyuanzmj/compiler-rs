use std::{collections::HashSet, mem, path::Path};

use crate::{
  compile::Template,
  ir::index::RootNode,
  transform::{TransformOptions, transform_jsx},
};
use oxc_allocator::{Allocator, TakeIn};
use oxc_ast::{
  NONE,
  ast::{
    Argument, BindingPatternKind, Expression, ImportOrExportKind, JSXChild, Program, Statement,
    VariableDeclarationKind,
  },
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SPAN, SourceType};
use oxc_transformer::Transformer;
use oxc_traverse::{Ancestor, Traverse, TraverseCtx, traverse_mut};

pub struct JsxTraverse<'a> {
  root: bool,
  interop: bool,
  allocator: &'a Allocator,
  source_text: &'a str,
  source_type: SourceType,
  templates: Vec<Template>,
  helpers: HashSet<String>,
  delegates: HashSet<String>,
}

impl<'a> JsxTraverse<'a> {
  pub fn new(
    allocator: &'a Allocator,
    templates: Vec<Template>,
    root: bool,
    interop: bool,
  ) -> Self {
    Self {
      root,
      interop,
      allocator,
      source_text: "",
      source_type: SourceType::jsx(),
      templates,
      helpers: HashSet::new(),
      delegates: HashSet::new(),
    }
  }

  pub fn traverse(mut self, program: &mut Program<'a>) -> Vec<Template> {
    let allocator = self.allocator;

    self.source_type = program.source_type;
    self.source_text = program.source_text;

    traverse_mut(
      &mut self,
      allocator,
      program,
      SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping(),
      (),
    );

    let options = &oxc_transformer::TransformOptions::default();
    if self.root {
      Transformer::new(
        allocator,
        Path::new(if self.source_type.is_typescript() {
          "index.tsx"
        } else {
          "index.jsx"
        }),
        options,
      )
      .build_with_scoping(
        SemanticBuilder::new()
          .build(program)
          .semantic
          .into_scoping(),
        program,
      );
    }
    self.templates
  }
}

impl<'a> Traverse<'a, ()> for JsxTraverse<'a> {
  fn enter_expression(
    &mut self,
    node: &mut Expression<'a>,
    ctx: &mut oxc_traverse::TraverseCtx<'a, ()>,
  ) {
    if !matches!(node, Expression::JSXElement(_) | Expression::JSXFragment(_)) {
      return;
    }
    if self.interop {
      for node in ctx.ancestors() {
        if let Ancestor::CallExpressionArguments(node) = node {
          let name = node.callee().span().source_text(self.source_text);
          if name == "defineVaporComponent" {
            break;
          } else if name == "defineComponent" {
            return;
          }
        }
        continue;
      }
    }

    let allocator = ctx.ast.allocator;
    let span = node.span();
    let mut is_fragment = false;
    let children = match node {
      Expression::JSXFragment(node) => {
        is_fragment = true;
        node.children.take_in(ctx.ast)
      }
      Expression::JSXElement(node) => oxc_allocator::Vec::from_array_in(
        [JSXChild::Element(oxc_allocator::Box::new_in(
          node.take_in(ctx.ast),
          ctx.ast.allocator,
        ))],
        ctx.ast.allocator,
      ),
      _ => oxc_allocator::Vec::new_in(ctx.ast.allocator),
    };
    let root = RootNode {
      children,
      is_fragment,
    };
    let source = &self.source_text[..span.end as usize];
    let options = TransformOptions::build(source, mem::take(&mut self.templates), self.interop);
    let result = transform_jsx(allocator, root, options);
    self.helpers.extend(result.helpers);
    self.delegates.extend(result.delegates);
    let code_boxed = format!("(() => {{{}}})()", result.code).into_boxed_str();
    let code_raw = Box::into_raw(code_boxed);
    let mut program = Parser::new(self.allocator, unsafe { &*code_raw }, self.source_type)
      .parse()
      .program;
    self.templates = JsxTraverse::new(self.allocator, result.templates, false, self.interop)
      .traverse(&mut program);
    if let Some(Statement::ExpressionStatement(stmt)) = &mut program.body.get_mut(0) {
      *node = stmt.expression.take_in(allocator);
    }
  }
  fn exit_program(&mut self, program: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
    if !self.root {
      return;
    };
    let ast = ctx.ast;
    let mut statements = vec![];
    if !self.delegates.is_empty() {
      statements.push(ast.statement_expression(
        SPAN,
        ast.expression_call(
          SPAN,
          ast.expression_identifier(SPAN, ast.atom("_delegateEvents")),
          NONE,
          oxc_allocator::Vec::from_iter_in(
            self.delegates.iter().map(|delegate| {
              Argument::StringLiteral(ctx.alloc(ast.string_literal(SPAN, ast.atom(delegate), None)))
            }),
            ast.allocator,
          ),
          false,
        ),
      ));
    }

    if !self.helpers.is_empty() {
      let helpers = vec![
        "setNodes",
        "createNodes",
        "createComponent",
        "createComponentWithFallback",
      ]
      .into_iter()
      .filter(|helper| {
        if self.helpers.contains(*helper) {
          self.helpers.remove(*helper);
          return true;
        } else {
          false
        }
      })
      .collect::<Vec<_>>();
      if !helpers.is_empty() {
        statements.push(Statement::ImportDeclaration(ast.alloc_import_declaration(
          SPAN,
          Some(ast.vec_from_iter(helpers.into_iter().map(|helper| {
            ast.import_declaration_specifier_import_specifier(
              SPAN,
              ast.module_export_name_identifier_name(SPAN, ast.atom(helper)),
              ast.binding_identifier(SPAN, ast.atom(format!("_{}", helper).as_str())),
              ImportOrExportKind::Value,
            )
          }))),
          ast.string_literal(SPAN, ast.atom("vue-jsx-vapor"), None),
          None,
          NONE,
          ImportOrExportKind::Value,
        )))
      }

      if !self.helpers.is_empty() {
        statements.push(Statement::ImportDeclaration(ast.alloc_import_declaration(
          SPAN,
          Some(ast.vec_from_iter(self.helpers.iter().map(|helper| {
            ast.import_declaration_specifier_import_specifier(
              SPAN,
              ast.module_export_name_identifier_name(SPAN, ast.atom(helper)),
              ast.binding_identifier(SPAN, ast.atom(format!("_{}", helper).as_str())),
              ImportOrExportKind::Value,
            )
          }))),
          ast.string_literal(SPAN, ast.atom("vue"), None),
          None,
          NONE,
          ImportOrExportKind::Value,
        )))
      }
    }

    let template_len = self.templates.len();
    if template_len > 0 {
      let template_statements = self
        .templates
        .iter()
        .enumerate()
        .map(|(index, template)| {
          let template_literal = Argument::StringLiteral(ctx.alloc(ast.string_literal(
            SPAN,
            ast.atom(&template.0),
            None,
          )));
          Statement::VariableDeclaration(ast.alloc(ast.variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            ast.vec1(ast.variable_declarator(
              SPAN,
              VariableDeclarationKind::Const,
              ast.binding_pattern(
                BindingPatternKind::BindingIdentifier(
                  ast.alloc(ast.binding_identifier(SPAN, ast.atom(format!("t{}", index).as_str()))),
                ),
                NONE,
                false,
              ),
              Some(ast.expression_call(
                SPAN,
                ast.expression_identifier(SPAN, ast.atom("_template")),
                NONE,
                if template.1 {
                  oxc_allocator::Vec::from_array_in(
                    [
                      template_literal,
                      Argument::BooleanLiteral(ast.alloc(ast.boolean_literal(SPAN, template.1))),
                    ],
                    ast.allocator,
                  )
                } else {
                  oxc_allocator::Vec::from_array_in([template_literal], ast.allocator)
                },
                false,
              )),
              false,
            )),
            false,
          )))
        })
        .collect::<Vec<_>>();
      statements.extend(template_statements);
    }

    if !statements.is_empty() {
      // Insert statements before the first non-import statement.
      let index = program
        .body
        .iter()
        .position(|stmt| !matches!(stmt, Statement::ImportDeclaration(_)))
        .unwrap_or(program.body.len());
      program.body.splice(index..index, statements);
    }
  }
}
