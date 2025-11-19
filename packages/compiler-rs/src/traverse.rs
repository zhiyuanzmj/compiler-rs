use std::path::Path;

use crate::{
  generate::{CodegenContext, generate},
  transform::{TransformContext, TransformOptions},
};
use oxc_allocator::{Allocator, CloneIn, TakeIn};
use oxc_ast::{
  NONE,
  ast::{
    Argument, BindingPatternKind, Expression, ImportOrExportKind, Program, Statement,
    VariableDeclarationKind,
  },
};
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SPAN, SourceType};
use oxc_transformer::Transformer;
use oxc_traverse::{Ancestor, Traverse, TraverseCtx, traverse_mut};

pub struct JsxTraverse<'a> {
  options: TransformOptions<'a>,
  allocator: &'a Allocator,
  source_text: &'a str,
  source_type: SourceType,
}

impl<'a> JsxTraverse<'a> {
  pub fn new(allocator: &'a Allocator, options: TransformOptions<'a>) -> Self {
    Self {
      options,
      allocator,
      source_text: "",
      source_type: SourceType::jsx(),
    }
  }

  pub fn traverse(mut self, program: &mut Program<'a>) {
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
    if self.options.interop {
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
    let source = &self.source_text[..span.end as usize];
    let transform_context =
      TransformContext::new(allocator, node.take_in(allocator), source, &self.options);
    transform_context.transform_node(None);
    let ir = &transform_context.ir.borrow();
    let block = transform_context.block.take();
    let generate_context = CodegenContext::new(allocator, ir, block, &self.options);
    let mut program = generate(&generate_context).clone_in(allocator);
    SemanticBuilder::new().build(&program);

    if let Some(Statement::ExpressionStatement(stmt)) = &mut program.body.get_mut(0) {
      *node = stmt.expression.take_in(allocator);
    }
  }
  fn exit_program(&mut self, program: &mut Program<'a>, ctx: &mut TraverseCtx<'a, ()>) {
    let ast = &ctx.ast;
    let mut statements = vec![];
    let delegates = self.options.delegates.take();
    if !delegates.is_empty() {
      statements.push(ast.statement_expression(
        SPAN,
        ast.expression_call(
          SPAN,
          ast.expression_identifier(SPAN, ast.atom("_delegateEvents")),
          NONE,
          oxc_allocator::Vec::from_iter_in(
            delegates.iter().map(|delegate| {
              Argument::StringLiteral(ctx.alloc(ast.string_literal(SPAN, ast.atom(delegate), None)))
            }),
            ast.allocator,
          ),
          false,
        ),
      ));
    }

    let mut helpers = self.options.helpers.take();
    if !helpers.is_empty() {
      let jsx_helpers = vec![
        "setNodes",
        "createNodes",
        "createComponent",
        "createComponentWithFallback",
      ]
      .into_iter()
      .filter(|helper| {
        if helpers.contains(*helper) {
          helpers.remove(*helper);
          return true;
        } else {
          false
        }
      })
      .collect::<Vec<_>>();
      if !jsx_helpers.is_empty() {
        statements.push(Statement::ImportDeclaration(ast.alloc_import_declaration(
          SPAN,
          Some(ast.vec_from_iter(jsx_helpers.into_iter().map(|helper| {
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

      if !helpers.is_empty() {
        statements.push(Statement::ImportDeclaration(ast.alloc_import_declaration(
          SPAN,
          Some(ast.vec_from_iter(helpers.iter().map(|helper| {
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

    let templates = self.options.templates.take();
    let template_len = templates.len();
    if template_len > 0 {
      let template_statements = templates
        .iter()
        .enumerate()
        .map(|(index, template)| {
          let template_literal =
            Argument::StringLiteral(ast.alloc_string_literal(SPAN, ast.atom(&template.0), None));

          Statement::VariableDeclaration(ast.alloc_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            ast.vec1(ast.variable_declarator(
              SPAN,
              VariableDeclarationKind::Const,
              ast.binding_pattern(
                BindingPatternKind::BindingIdentifier(
                  ast.alloc_binding_identifier(SPAN, ast.atom(&format!("t{index}"))),
                ),
                NONE,
                false,
              ),
              Some(ast.expression_call(
                SPAN,
                ast.expression_identifier(SPAN, ast.atom("_template")),
                NONE,
                if template.1 {
                  ast.vec_from_array([
                    template_literal,
                    Argument::BooleanLiteral(ast.alloc_boolean_literal(SPAN, template.1)),
                  ])
                } else {
                  oxc_allocator::Vec::from_array_in([template_literal], ast.allocator)
                },
                false,
              )),
              false,
            )),
            false,
          ))
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
