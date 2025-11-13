use std::path::Path;

use crate::{
  generate::VaporCodegenResult,
  ir::index::RootNode,
  transform::{TransformOptions, transform_jsx},
};

use napi::{
  Env,
  bindgen_prelude::{Function, Object},
};
use napi_derive::napi;
use oxc_allocator::{Allocator, TakeIn};
use oxc_ast::ast::{Expression, JSXChild, Statement};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

#[cfg_attr(feature = "napi", napi)]
pub type Template = (String, bool);

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Default)]
pub struct CompilerOptions {
  pub source: Option<String>,
  pub templates: Option<Vec<Template>>,
  /**
   * Whether to compile components to createComponentWithFallback.
   * @default false
   */
  pub with_fallback: Option<bool>,
  /**
   * Separate option for end users to extend the native elements list
   */
  pub is_custom_element: Option<Function<'static, String, bool>>,
  pub on_error: Option<Function<'static, Object<'static>, ()>>,
  /**
   * Generate source map?
   * @default false
   */
  pub source_map: Option<bool>,
  /**
   * Filename for source map generation.
   * Also used for self-recursive reference in templates
   * @default 'index.jsx'
   */
  pub filename: Option<String>,
  /**
   * When enabled, JSX within `defineVaporComponent` is transformed to Vapor DOM,
   * while all other JSX is transformed to Virtual DOM.
   */
  pub interop: Option<bool>,
}

#[cfg(feature = "napi")]
#[napi]
pub fn _compile(env: Env, source: String, options: Option<CompilerOptions>) -> VaporCodegenResult {
  use crate::utils::error::ErrorCodes;
  let options = options.unwrap_or_default();
  compile(
    "",
    Some(TransformOptions {
      source: &options.source.unwrap_or(source),
      filename: &options.filename.unwrap_or("index.jsx".to_string()),
      templates: options.templates.unwrap_or(vec![]),
      source_map: options.source_map.unwrap_or(false),
      with_fallback: options.with_fallback.unwrap_or(false),
      interop: options.interop.unwrap_or(false),
      is_custom_element: if let Some(is_custom_element) = options.is_custom_element {
        Box::new(move |tag: String| is_custom_element.call(tag).unwrap())
          as Box<dyn Fn(String) -> bool>
      } else {
        Box::new(|_: String| false) as Box<dyn Fn(String) -> bool>
      },
      on_error: if let Some(on_error) = options.on_error {
        use crate::utils::error::create_compiler_error;

        Box::new(move |code: ErrorCodes| {
          let compiler_error = create_compiler_error(&env, code, None).unwrap();
          on_error.call(compiler_error).unwrap();
        }) as Box<dyn Fn(ErrorCodes)>
      } else {
        Box::new(|_: ErrorCodes| {}) as Box<dyn Fn(ErrorCodes)>
      },
    }),
  )
}

pub fn compile(source: &str, options: Option<TransformOptions>) -> VaporCodegenResult {
  let options = options.unwrap_or(TransformOptions::build(source, vec![], false));
  let source_type = SourceType::from_path(Path::new(&options.filename)).unwrap();
  let allocator = Allocator::default();
  let mut root = Parser::new(&allocator, &options.source, source_type)
    .with_options(ParseOptions {
      parse_regular_expression: true,
      ..ParseOptions::default()
    })
    .parse();
  let Statement::ExpressionStatement(stmt) = root.program.body.get_mut(0).unwrap() else {
    panic!("Expected ExpressionStatement");
  };
  let mut is_fragment = false;
  let children = match &mut stmt.expression {
    Expression::JSXFragment(node) => {
      is_fragment = true;
      node.children.take_in(&allocator)
    }
    Expression::JSXElement(node) => oxc_allocator::Vec::from_array_in(
      [JSXChild::Element(oxc_allocator::Box::new_in(
        node.take_in(&allocator),
        &allocator,
      ))],
      &allocator,
    ),
    _ => oxc_allocator::Vec::new_in(&allocator),
  };
  let root = RootNode {
    is_fragment,
    children,
  };

  transform_jsx(&allocator, root, options)
}
