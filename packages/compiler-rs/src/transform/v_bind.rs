use napi::{
  Result,
  bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  transform::DirectiveTransformResult,
  utils::{
    check::is_reserved_prop,
    expression::{create_simple_expression, resolve_expression},
    text::camelize,
  },
};

#[napi]
pub fn transform_v_bind(
  #[napi(ts_arg_type = "import('oxc-parser').JSXAttribute")] dir: Object,
  #[napi(ts_arg_type = "import('oxc-parser').JSXElement")] _: Object,
  context: Object,
) -> Result<Option<DirectiveTransformResult>> {
  let name = dir.get_named_property::<Object>("name")?;
  let value = dir.get_named_property::<Object>("value");

  let name_string = name.get_named_property::<String>("name")?;
  let name_splited: Vec<&str> = name_string.split("_").collect();
  let modifiers = name_splited[1..].to_vec();
  let name_string = name_splited[0].to_string();

  let exp = if let Ok(value) = value {
    resolve_expression(value, context)
  } else {
    create_simple_expression(String::from("true"), None, None, None)
  };

  let mut arg = create_simple_expression(name_string, Some(true), Some(name), None);
  if is_reserved_prop(&arg.content) {
    return Ok(None);
  }

  if modifiers.contains(&"camel") {
    arg.content = camelize(arg.content)
  }

  let modifier = if modifiers.contains(&"prop") {
    Some(String::from("."))
  } else if modifiers.contains(&"attr") {
    Some(String::from("^"))
  } else {
    None
  };
  dbg!(&exp.content);

  Ok(Some(DirectiveTransformResult {
    key: arg,
    value: exp,
    runtime_camelize: Some(false),
    modifier,
    handler: None,
    handler_modifiers: None,
    model: None,
    model_modifiers: None,
  }))
}
