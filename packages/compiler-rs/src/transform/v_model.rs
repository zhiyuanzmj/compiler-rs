use std::rc::Rc;

use napi::{
  Either, Result,
  bindgen_prelude::{Either16, JsObjectValue, Object},
};

use crate::{
  ir::index::{BlockIRNode, DirectiveIRNode, IRNodeTypes},
  transform::{DirectiveTransformResult, TransformContext},
  utils::{
    check::{is_jsx_component, is_member_expression, is_string_literal},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::create_simple_expression,
    text::get_text,
    utils::find_prop,
  },
};

pub fn transform_v_model(
  _dir: Object,
  node: Object,
  context: &Rc<TransformContext>,
  context_block: &mut BlockIRNode,
) -> Result<Option<DirectiveTransformResult>> {
  let dir = resolve_directive(_dir, context)?;

  let Some(exp) = &dir.exp else {
    on_error(ErrorCodes::X_V_MODEL_NO_EXPRESSION, context);
    return Ok(None);
  };

  let exp_string = &exp.content;
  if exp_string.trim().is_empty() || !is_member_expression(exp) {
    on_error(ErrorCodes::X_V_MODEL_MALFORMED_EXPRESSION, context);
    return Ok(None);
  }

  let is_component = is_jsx_component(node);
  if is_component {
    return Ok(Some(DirectiveTransformResult {
      key: if let Some(arg) = dir.arg {
        arg
      } else {
        create_simple_expression("modelValue".to_string(), Some(true), None, None)
      },
      value: dir.exp.unwrap(),
      model: Some(true),
      model_modifiers: Some(
        dir
          .modifiers
          .iter()
          .map(|m| m.content.to_string())
          .collect(),
      ),
      handler: None,
      handler_modifiers: None,
      modifier: None,
      runtime_camelize: None,
    }));
  }

  if dir.arg.is_some() {
    on_error(ErrorCodes::X_V_MODEL_ARG_ON_ELEMENT, context);
  }

  let tag = get_text(
    node
      .get_named_property::<Object>("openingElement")?
      .get_named_property::<Object>("name")?,
    context,
  );
  let is_custom_element = context.options.is_custom_element.call(tag.clone())?;
  let mut model_type = "text";
  // TODO let runtimeDirective: VaporHelper | undefined = 'vModelText'
  if matches!(tag.as_str(), "input" | "textarea" | "select") || is_custom_element {
    if tag == "input" || is_custom_element {
      let _type = find_prop(&node, Either::A("type".to_string()));
      if let Some(_type) = _type {
        let value = _type.get_named_property::<Object>("value")?;
        if value
          .get_named_property::<String>("type")?
          .eq("JSXExpressionContainer")
        {
          // type={foo}
          model_type = "dynamic"
        } else if is_string_literal(Some(value)) {
          let value = value.get_named_property::<String>("value")?;
          match value.as_str() {
            "radio" => model_type = "radio",
            "checkbox" => model_type = "checkbox",
            "file" => {
              model_type = "";
              on_error(ErrorCodes::X_V_MODEL_ON_FILE_INPUT_ELEMENT, context);
            }
            // text type
            _ => check_duplicated_value(node, context),
          }
        }
      } else if has_dynamic_key_v_bind(node)? {
        // element has bindings with dynamic keys, which can possibly contain "type".
        model_type = "dynamic";
      } else {
        // text type
        check_duplicated_value(node, context)
      }
    } else if tag == "select" {
      model_type = "select"
    } else {
      // textarea
      check_duplicated_value(node, context)
    }
  } else {
    on_error(ErrorCodes::X_V_MODEL_ON_INVALID_ELEMENT, context)
  }

  if !model_type.is_empty() {
    let element = context.reference(&mut context_block.dynamic)?;
    context.register_operation(
      context_block,
      Either16::M(DirectiveIRNode {
        directive: true,
        _type: IRNodeTypes::DIRECTIVE,
        element,
        dir,
        name: "model".to_string(),
        model_type: Some(model_type.to_string()),
        builtin: Some(true),
        asset: None,
      }),
      None,
    )?
  }

  Ok(None)
}

fn check_duplicated_value(node: Object, context: &Rc<TransformContext>) {
  let value = find_prop(&node, Either::A("value".to_string()));
  if let Some(value) = value
    && !is_string_literal(value.get_named_property::<Object>("value").ok())
  {
    on_error(ErrorCodes::X_V_MODEL_UNNECESSARY_VALUE, context);
  }
}

fn has_dynamic_key_v_bind(node: Object) -> Result<bool> {
  Ok(
    node
      .get_named_property::<Object>("openingElement")?
      .get_named_property::<Vec<Object>>("attributes")?
      .iter()
      .any(|p| {
        let _type = p.get_named_property::<String>("type").unwrap();
        if _type == "JSXSpreadAttribute" {
          true
        } else if _type == "JSXAttribute" {
          let name = p.get_named_property::<Object>("name").unwrap();
          name
            .get_named_property::<String>("type")
            .unwrap()
            .eq("JSXNamespacedName")
            && !name
              .get_named_property::<Object>("namespace")
              .unwrap()
              .get_named_property::<String>("name")
              .unwrap()
              .starts_with("v-")
        } else {
          false
        }
      }),
  )
}
