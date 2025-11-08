use std::path::Path;

use napi::{
  Env,
  bindgen_prelude::{Function, Object},
};
use napi_derive::napi;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, JSXChild, Statement};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

use crate::{
  generate::VaporCodegenResult,
  ir::index::RootNode,
  transform::{TransformOptions, transform},
};

#[napi(object)]
pub struct CompilerOptions {
  pub source: Option<String>,
  pub templates: Option<Vec<String>>,
  /**
   * Whether to compile components to createComponentWithFallback.
   * @default false
   */
  pub with_fallback: Option<bool>,
  /**
   * Indicates that transforms and codegen should try to output valid TS code
   */
  #[napi(js_name = "isTS")]
  pub is_ts: Option<bool>,
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
}

#[napi]
pub fn compile(env: Env, source: String, options: Option<CompilerOptions>) -> VaporCodegenResult {
  let mut options = options.unwrap_or(CompilerOptions {
    source: None,
    filename: None,
    is_ts: None,
    on_error: None,
    is_custom_element: None,
    templates: None,
    source_map: None,
    with_fallback: None,
  });
  let resolved_options = TransformOptions {
    source: options.source.unwrap_or(source),
    filename: options.filename.unwrap_or("index.jsx".to_string()),
    templates: options.templates.unwrap_or(vec![]),
    source_map: options.source_map.unwrap_or(false),
    is_ts: options.is_ts.unwrap_or(false),
    with_fallback: options.with_fallback.unwrap_or(false),
    is_custom_element: if let Some(is_custom_element) = options.is_custom_element {
      Box::new(move |tag: String| is_custom_element.call(tag).unwrap())
        as Box<dyn Fn(String) -> bool>
    } else {
      Box::new(|_: String| false) as Box<dyn Fn(String) -> bool>
    },
    on_error: if let Some(on_error) = options.on_error.take() {
      Box::new(move |error: Object| on_error.call(error).unwrap()) as Box<dyn Fn(Object)>
    } else {
      Box::new(|_: Object| {}) as Box<dyn Fn(Object)>
    },
  };

  let source_type = SourceType::from_path(Path::new(&resolved_options.filename)).unwrap();
  let allocator = Allocator::default();
  let root = Parser::new(&allocator, &resolved_options.source, source_type)
    .with_options(ParseOptions {
      parse_regular_expression: true,
      ..ParseOptions::default()
    })
    .parse();
  let Statement::ExpressionStatement(stmt) = root.program.body.get(0).unwrap() else {
    panic!("Expected ExpressionStatement");
  };
  let mut is_fragment = false;
  let children = match &stmt.expression {
    Expression::JSXFragment(j) => {
      is_fragment = true;
      j.children.clone_in(&allocator)
    }
    Expression::JSXElement(j) => {
      let mut arr = oxc_allocator::Vec::new_in(&allocator);
      arr.push(JSXChild::Element(j.clone_in(&allocator)));
      arr
    }
    _ => oxc_allocator::Vec::new_in(&allocator),
  };
  let root = RootNode {
    children,
    is_fragment,
  };

  transform(env, &allocator, root, resolved_options)
}
