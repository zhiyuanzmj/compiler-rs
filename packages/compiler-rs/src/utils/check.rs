use napi::bindgen_prelude::{BigInt, Object};
use napi_derive::napi;

macro_rules! def_literal_checker {
  ($name:ident, $type:ty, $ts_return_type: literal) => {
    #[napi(ts_args_type = "node?: import('oxc-parser').Node | undefined | null", ts_return_type = $ts_return_type)]
    pub fn $name(node: Option<Object>) -> bool {
      let Some(node) = node else { return false };
      if let Ok(Some(type_value)) = node.get::<String>("type") {
        type_value.eq("Literal") && matches!(node.get::<$type>("value"), Ok(Some(_)))
      } else {
        false
      }
    }
  };
}

def_literal_checker!(
  is_string_literal,
  String,
  "node is import('oxc-parser').StringLiteral"
);
def_literal_checker!(
  is_big_int_literal,
  BigInt,
  "node is import('oxc-parser').BigIntLiteral"
);
def_literal_checker!(
  is_numeric_literal,
  f64,
  "node is import('oxc-parser').NumericLiteral"
);

#[napi(ts_args_type = "node?: import('oxc-parser').Node | undefined | null")]
pub fn is_template(node: Option<Object>) -> bool {
  let Some(node) = node else { return false };
  if !matches!(node.get::<String>("type"), Ok(Some(type_value)) if type_value.eq("JSXElement")) {
    return false;
  };
  if let Some(name) = node
    .get::<Object>("openingElement")
    .ok()
    .flatten()
    .and_then(|elem| elem.get::<Object>("name").ok().flatten())
  {
    matches!(name.get::<String>("type"), Ok(Some(type_value)) if type_value == "JSXIdentifier")
      && matches!(name.get::<String>("name"), Ok(Some(name)) if name == "template")
  } else {
    false
  }
}
