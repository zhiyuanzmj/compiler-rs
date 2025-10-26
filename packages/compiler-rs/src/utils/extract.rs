use napi::{
  JsValue, Result, ValueType,
  bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;

#[napi]
pub fn extract_identifiers(
  node: Object<'static>,
  mut identifiers: Vec<Object<'static>>,
) -> Result<Vec<Object<'static>>> {
  match node.get_named_property::<String>("type")?.as_str() {
    "Identifier" | "JSXIdentifier" => identifiers.push(node),
    "MemberExpression" | "JSXMemberExpression" => {
      let mut object = node;
      while object
        .get_named_property::<String>("type")?
        .eq("MemberExpression")
      {
        object = object.get_named_property::<Object>("object")?;
      }
      identifiers.push(object)
    }
    "ObjectPattern" => {
      for prop in node.get_named_property::<Vec<Object>>("properties")? {
        identifiers = if prop.get_named_property::<String>("type")?.eq("RestElement") {
          extract_identifiers(prop.get_named_property::<Object>("argument")?, identifiers)?
        } else {
          extract_identifiers(prop.get_named_property::<Object>("value")?, identifiers)?
        }
      }
    }
    "ArrayPattern" => {
      for element in node.get_named_property::<Vec<Object>>("elements")? {
        if !matches!(element.to_unknown().get_type(), Ok(ValueType::Null)) {
          identifiers = extract_identifiers(element, identifiers)?;
        }
      }
    }
    "RestElement" => {
      identifiers =
        extract_identifiers(node.get_named_property::<Object>("argument")?, identifiers)?;
    }
    "AssignmentPattern" => {
      identifiers = extract_identifiers(node.get_named_property::<Object>("left")?, identifiers)?;
    }
    _ => (),
  }
  Ok(identifiers)
}
