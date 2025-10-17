use std::{collections::HashSet, sync::LazyLock};

use napi::{
  JsValue, ValueType,
  bindgen_prelude::{BigInt, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::SimpleExpressionNode,
  utils::{expression::is_globally_allowed, utils::unwrap_ts_node},
};

#[napi]
pub fn _is_member_expression(exp: SimpleExpressionNode) -> bool {
  is_member_expression(&exp)
}

pub fn is_member_expression(exp: &SimpleExpressionNode) -> bool {
  let Some(ast) = exp.ast else { return false };
  let ret = unwrap_ts_node(ast);
  let _type = ret.get_named_property::<String>("type").unwrap();
  _type == "MemberExpression"
    || (_type == "Identifier"
      && !ret
        .get_named_property::<String>("name")
        .unwrap()
        .eq("undefined"))
}

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
#[napi]
pub fn is_constant_node(node: Option<Object>) -> bool {
  _is_constant_node(&node)
}
pub fn _is_constant_node(node: &Option<Object>) -> bool {
  let Some(node) = node else {
    return false;
  };
  let node = unwrap_ts_node(*node);
  let Some(node_type) = node.get_named_property::<String>("type").ok() else {
    return false;
  };
  if node_type == "UnaryExpression" {
    // void 0, !true
    is_constant_node(node.get::<Object>("argument").unwrap_or(None))
  } else if node_type == "LogicalExpression" || node_type == "BinaryExpression" {
    // 1 > 2, // 1 + 2
    is_constant_node(node.get::<Object>("left").unwrap_or(None))
      && is_constant_node(node.get::<Object>("right").unwrap_or(None))
  } else if node_type == "ConditionalExpression" {
    // 1 ? 2 : 3
    is_constant_node(node.get::<Object>("test").unwrap_or(None))
      && is_constant_node(node.get::<Object>("consequent").unwrap_or(None))
      && is_constant_node(node.get::<Object>("alternate").unwrap_or(None))
  } else if node_type == "SequenceExpression" || node_type == "TemplateLiteral" {
    // (1, 2) | `foo${1}`
    node
      .get::<Vec<Object>>("expressions")
      .unwrap()
      .unwrap()
      .into_iter()
      .all(|exp| is_constant_node(Some(exp)))
  } else if node_type == "ParenthesizedExpression" {
    is_constant_node(node.get::<Object>("expression").unwrap_or(None))
  } else if node_type == "Literal" {
    true
  } else if node_type == "Identifier" {
    let name = node
      .get_named_property::<String>("name")
      .unwrap_or(String::new());
    // .is_ok_and(|name| name.unwrap_or(String::new()).eq("undefined"));
    name == "undefined" || is_globally_allowed(&name)
  } else if node_type == "ObjectExpression" {
    let Some(props) = node.get_named_property::<Vec<Object>>("properties").ok() else {
      return false;
    };
    props.iter().all(|prop| {
      let name_type = prop
        .get_named_property::<String>("type")
        .unwrap_or(String::new());
      // { bar() {} } object methods are not considered static nodes
      if name_type == "Property"
        && prop
          .get_named_property::<bool>("method")
          .is_ok_and(|m| m == true)
      {
        return false;
      }
      // { ...{ foo: 1 } }
      if name_type == "SpreadElement" {
        return is_constant_node(prop.get_named_property::<Object>("argument").ok());
      }
      // { foo: 1 }
      (prop
        .get_named_property::<bool>("computed")
        .is_ok_and(|m| m != true)
        || is_constant_node(prop.get_named_property::<Object>("key").ok()))
        && is_constant_node(prop.get_named_property("value").ok())
    })
  } else if node_type == "ArrayExpression" {
    let Some(elements) = node.get_named_property::<Vec<Object>>("elements").ok() else {
      return false;
    };
    elements.iter().all(|element| {
      // [1, , 3]
      if let Ok(ValueType::Null) = element.to_unknown().get_type() {
        return true;
      }
      // [1, ...[2, 3]]
      if element
        .get_named_property::<String>("type")
        .is_ok_and(|t| t == "SpreadElement")
      {
        return is_constant_node(element.get_named_property("argument").ok());
      }
      // [1, 2]
      is_constant_node(Some(element.to_owned()))
    })
  } else {
    false
  }
}

// https://developer.mozilla.org/en-US/docs/Web/HTML/Element
static HTML_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
  HashSet::from([
    "html",
    "body",
    "base",
    "head",
    "link",
    "meta",
    "style",
    "title",
    "address",
    "article",
    "aside",
    "footer",
    "header",
    "hgroup",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "nav",
    "section",
    "div",
    "dd",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "picture",
    "hr",
    "img",
    "li",
    "main",
    "ol",
    "p",
    "pre",
    "ul",
    "a",
    "b",
    "abbr",
    "bdi",
    "bdo",
    "br",
    "cite",
    "code",
    "data",
    "dfn",
    "em",
    "i",
    "kbd",
    "mark",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "small",
    "span",
    "strong",
    "sub",
    "sup",
    "time",
    "u",
    "var",
    "wbr",
    "area",
    "audio",
    "map",
    "track",
    "video",
    "embed",
    "object",
    "param",
    "source",
    "canvas",
    "script",
    "noscript",
    "del",
    "ins",
    "caption",
    "col",
    "colgroup",
    "table",
    "thead",
    "tbody",
    "td",
    "th",
    "tr",
    "button",
    "datalist",
    "fieldset",
    "form",
    "input",
    "label",
    "legend",
    "meter",
    "optgroup",
    "option",
    "output",
    "progress",
    "select",
    "textarea",
    "details",
    "dialog",
    "menu",
    "summary",
    "template",
    "blockquote",
    "iframe",
    "tfoot",
  ])
});
pub fn is_html_tag(tag_name: &str) -> bool {
  HTML_TAGS.contains(tag_name)
}

// https://developer.mozilla.org/en-US/docs/Web/SVG/Element
static SVG_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
  HashSet::from([
    "svg",
    "animate",
    "animateMotion",
    "animateTransform",
    "circle",
    "clipPath",
    "color-profile",
    "defs",
    "desc",
    "discard",
    "ellipse",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
    "filter",
    "foreignObject",
    "g",
    "hatch",
    "hatchpath",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "mesh",
    "meshgradient",
    "meshpatch",
    "meshrow",
    "metadata",
    "mpath",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "set",
    "solidcolor",
    "stop",
    "switch",
    "symbol",
    "text",
    "textPath",
    "title",
    "tspan",
    "unknown",
    "use",
    "view",
  ])
});
pub fn is_svg_tag(tag_name: &str) -> bool {
  SVG_TAGS.contains(tag_name)
}

#[napi(
  js_name = "isJSXComponent",
  ts_args_type = "node: import('oxc-parser').Node"
)]
pub fn is_jsx_component(node: Object) -> bool {
  if node
    .get_named_property::<String>("type")
    .is_ok_and(|t| t != "JSXElement")
  {
    return false;
  }

  let Ok(Some(name)) = node
    .get_named_property::<Object>("openingElement")
    .map(|obj| obj.get_named_property::<Object>("name").ok())
  else {
    return false;
  };
  let Ok(name_type) = name.get_named_property::<String>("type") else {
    return false;
  };
  if name_type == "JSXIdentifier" {
    name
      .get_named_property::<String>("name")
      .is_ok_and(|name| !is_html_tag(&name) && !is_svg_tag(&name))
  } else {
    name_type == "JSXMemberExpression"
  }
}

#[napi(
  ts_args_type = "node: import('oxc-parser').Node | RootNode",
  ts_return_type = "node is import('oxc-parser').JSXElement | import('oxc-parser').JSXFragment | RootNode"
)]
pub fn is_fragment_node(node: Object) -> bool {
  if let Ok(node_type) = node.get_named_property::<String>("type") {
    return node_type == "JSXFragment" || node_type == "ROOT" || is_template(Some(node));
  }
  return false;
}

static RESERVED_PROP: [&str; 4] = ["key", "ref", "ref_for", "ref_key"];
pub fn is_reserved_prop(prop_name: &str) -> bool {
  RESERVED_PROP.contains(&prop_name)
}

static VOID_TAGS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
  HashSet::from([
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
  ])
});
pub fn is_void_tag(tag_name: &str) -> bool {
  VOID_TAGS.contains(&tag_name)
}

static BUILD_IN_DIRECTIVE: [&str; 15] = [
  "bind", "cloak", "else-if", "else", "for", "html", "if", "model", "on", "once", "pre", "show",
  "slot", "text", "memo",
];
pub fn is_build_in_directive(prop_name: &str) -> bool {
  BUILD_IN_DIRECTIVE.contains(&prop_name)
}
