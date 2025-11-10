use std::mem;

use napi::Either;
use napi::bindgen_prelude::Either3;
use napi::bindgen_prelude::Either4;

use crate::generate::CodegenContext;
use crate::generate::expression::gen_expression;
use crate::generate::utils::CodeFragment;
use crate::generate::utils::FragmentSymbol::Newline;
use crate::generate::utils::NewlineType;
use crate::generate::utils::gen_call;
use crate::generate::utils::gen_multi;
use crate::generate::utils::get_delimiters_array;
use crate::generate::utils::get_delimiters_object;
use crate::ir::component::IRProp;
use crate::ir::index::SetDynamicPropsIRNode;
use crate::ir::index::SetPropIRNode;
use crate::ir::index::SimpleExpressionNode;
use crate::utils::check::is_simple_identifier;
use crate::utils::check::is_svg_tag;

pub struct HelperConfig {
  name: String,
  need_key: bool,
}

fn helpers(name: &str) -> HelperConfig {
  match name {
    "setText" => HelperConfig {
      name: "setText".to_string(),
      need_key: false,
    },
    "setHtml" => HelperConfig {
      name: "setHtml".to_string(),
      need_key: false,
    },
    "setClass" => HelperConfig {
      name: "setClass".to_string(),
      need_key: false,
    },
    "setStyle" => HelperConfig {
      name: "setStyle".to_string(),
      need_key: false,
    },
    "setValue" => HelperConfig {
      name: "setValue".to_string(),
      need_key: false,
    },
    "setAttr" => HelperConfig {
      name: "setAttr".to_string(),
      need_key: true,
    },
    "setProp" => HelperConfig {
      name: "setProp".to_string(),
      need_key: true,
    },
    "setDOMProp" => HelperConfig {
      name: "setDOMProp".to_string(),
      need_key: true,
    },
    "setDynamicProps" => HelperConfig {
      name: "setDynamicProps".to_string(),
      need_key: true,
    },
    _ => panic!("Unsupported helper name"),
  }
}

pub fn gen_set_prop(oper: SetPropIRNode, context: &CodegenContext) -> Vec<CodeFragment> {
  let SetPropIRNode {
    prop: IRProp {
      key,
      values,
      modifier,
      ..
    },
    tag,
    ..
  } = oper;
  let resolved_helper = get_runtime_helper(&tag, &key.content, modifier);
  let prop_value = gen_prop_value(values, context);
  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::B((context.helper(resolved_helper.name.as_str()), None)),
    vec![
      Either4::C(Some(format!("n{}", oper.element))),
      if resolved_helper.need_key {
        Either4::D(gen_expression(key, context, None, None))
      } else {
        Either4::C(None)
      },
      Either4::D(prop_value),
    ],
  ));
  result
}

fn get_runtime_helper(tag: &str, key: &str, modifier: Option<String>) -> HelperConfig {
  let tag_name = tag.to_uppercase();
  if let Some(modifier) = modifier {
    return if modifier.eq(".") {
      if let Some(result) = get_special_helper(key, &tag_name) {
        result
      } else {
        helpers("setDOMProp")
      }
    } else {
      helpers("setAttr")
    };
  }

  // 1. special handling for value / style / class / textContent /  innerHTML
  if let Some(helper) = get_special_helper(key, &tag_name) {
    return helper;
  };

  // 2. Aria DOM properties shared between all Elements in
  //    https://developer.mozilla.org/en-US/docs/Web/API/Element
  if key.starts_with("aria")
    && key
      .chars()
      .nth(4)
      .map(|c| c.is_ascii_uppercase())
      .unwrap_or(false)
  {
    return helpers("setDOMProp");
  }

  // 3. SVG: always attribute
  if is_svg_tag(tag) {
    // TODO pass svg flag
    return helpers("setAttr");
  }

  // 4. respect shouldSetAsAttr used in vdom and setDynamicProp for consistency
  //    also fast path for presence of hyphen (covers data-* and aria-*)
  if should_set_as_attr(&tag_name, key) || key.contains("-") {
    return helpers("setAttr");
  }

  // 5. Fallback to setDOMProp, which has a runtime `key in el` check to
  // ensure behavior consistency with vdom
  return helpers("setProp");
}

// The following attributes must be set as attribute
fn should_set_as_attr(tag_name: &str, key: &str) -> bool {
  // these are enumerated attrs, however their corresponding DOM properties
  // are actually booleans - this leads to setting it with a string "false"
  // value leading it to be coerced to `true`, so we need to always treat
  // them as attributes.
  // Note that `contentEditable` doesn't have this problem: its DOM
  // property is also enumerated string values.
  if key == "spellcheck" || key == "draggable" || key == "translate" || key == "autocorrect" {
    return true;
  }

  // #1787, #2840 form property on form elements is readonly and must be set as attribute.
  if key == "form" {
    return true;
  }

  // #1526 <input list> must be set as attribute
  if key == "list" && tag_name == "INPUT" {
    return true;
  }

  // #8780 the width or height of embedded tags must be set as attribute
  if (key == "width" || key == "height")
    && (tag_name == "IMG" || tag_name == "VIDEO" || tag_name == "CANVAS" || tag_name == "SOURCE")
  {
    return true;
  }

  return false;
}

fn can_set_value_directly(tag_name: &str) -> bool {
  tag_name != "PROGRESS" &&
    // custom elements may use _value internally
    !tag_name.contains("-")
}

fn get_special_helper(key_name: &str, tag_name: &str) -> Option<HelperConfig> {
  // special case for 'value' property
  match key_name {
    "value" if can_set_value_directly(tag_name) => Some(helpers("setValue")),
    "class" => Some(helpers("setClass")),
    "style" => Some(helpers("setStyle")),
    "innerHTML" => Some(helpers("setHtml")),
    "textContent" => Some(helpers("setText")),
    _ => None,
  }
}

// dynamic key props and {...obj} will reach here
pub fn gen_dynamic_props(
  oper: SetDynamicPropsIRNode,
  context: &CodegenContext,
) -> Vec<CodeFragment> {
  let values = oper
    .props
    .into_iter()
    .map(|props| {
      Either4::D(match props {
        // static and dynamic arg props
        Either3::A(props) => gen_literal_object_props(props, context), // static and dynamic arg props
        Either3::B(props) => gen_literal_object_props(vec![props], context), // dynamic arg props
        Either3::C(props) => gen_expression(props.value, context, None, None), // {...obj}
      })
    })
    .collect::<Vec<_>>();

  let mut result = vec![Either3::A(Newline)];
  result.extend(gen_call(
    Either::A(context.helper("setDynamicProps")),
    vec![
      Either4::C(Some(format!("n{}", oper.element))),
      Either4::D(gen_multi(get_delimiters_array(), values)),
      Either4::C(if oper.root {
        Some("true".to_string())
      } else {
        None
      }),
    ],
  ));
  result
}

fn gen_literal_object_props(props: Vec<IRProp>, context: &CodegenContext) -> Vec<CodeFragment> {
  gen_multi(
    get_delimiters_object(),
    props
      .into_iter()
      .map(|mut prop| {
        let values = mem::take(&mut prop.values);
        let mut result = gen_prop_key(prop, context);
        result.push(Either3::C(Some(": ".to_string())));
        result.extend(gen_prop_value(values, context));
        Either4::D(result)
      })
      .collect::<Vec<_>>(),
  )
}

pub fn gen_prop_key(oper: IRProp, context: &CodegenContext) -> Vec<CodeFragment> {
  let IRProp {
    key: node,
    modifier,
    runtime_camelize,
    handler,
    handler_modifiers,
    ..
  } = oper;

  let handler_modifier_postfix = if let Some(handler_modifiers) = handler_modifiers {
    handler_modifiers
      .options
      .into_iter()
      .map(|option| option[..1].to_string().to_uppercase() + &option[1..].to_string())
      .collect::<Vec<_>>()
      .join("")
  } else {
    String::new()
  };
  // static arg was transformed by v-bind transformer
  if node.is_static {
    // only quote keys if necessary
    let key_name = (if handler.unwrap_or(false) {
      format!(
        "on{}",
        node.content[0..1].to_string().to_uppercase() + &node.content[1..].to_string()
      )
    } else {
      node.content
    }) + &handler_modifier_postfix;
    return vec![Either3::B((
      if is_simple_identifier(&key_name) {
        key_name
      } else {
        format!("\"{}\"", key_name)
      },
      NewlineType::None,
      node.loc,
      None,
    ))];
  }

  let mut key = gen_expression(node, context, None, None);
  if runtime_camelize.unwrap_or_default() {
    key = gen_call(Either::A(context.helper("camelize")), vec![Either4::D(key)])
  }
  if handler.unwrap_or_default() {
    key = gen_call(
      Either::A(context.helper("toHandlerKey")),
      vec![Either4::D(key)],
    )
  }
  let mut result = vec![
    Either3::C(Some("[".to_string())),
    Either3::C(if let Some(modifier) = modifier {
      Some(format!("\"{}\" + ", modifier))
    } else {
      None
    }),
  ];
  result.extend(key);
  result.push(Either3::C(if !handler_modifier_postfix.is_empty() {
    Some(format!(" + \"{}\"", handler_modifier_postfix))
  } else {
    None
  }));
  result.push(Either3::C(Some("]".to_string())));
  result
}

pub fn gen_prop_value(
  mut values: Vec<SimpleExpressionNode>,
  context: &CodegenContext,
) -> Vec<CodeFragment> {
  if (&values).len() == 1 {
    return gen_expression(values.remove(0), context, None, None);
  }
  gen_multi(
    get_delimiters_array(),
    values
      .into_iter()
      .map(|expr| Either4::D(gen_expression(expr, context, None, None)))
      .collect::<Vec<_>>(),
  )
}
