use std::{
  collections::{HashMap, HashSet},
  sync::LazyLock,
};

use napi::{
  Env, JsValue, Result, ValueType,
  bindgen_prelude::{BigInt, FnArgs, Function, JsObjectValue, JsValuesTuple, Object},
};
use napi_derive::napi;
use regex::Regex;

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

pub fn is_template(node: &Object) -> bool {
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

pub fn is_fragment_node(node: &Object) -> bool {
  if let Ok(node_type) = node.get_named_property::<String>("type") {
    return node_type == "JSXFragment" || node_type == "ROOT" || is_template(&node);
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

static NON_IDENTIFIER_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^$|^\d|[^$\w\x{00A0}-\x{FFFF}]").unwrap());
#[napi]
pub fn _is_simple_identifier(name: String) -> bool {
  !NON_IDENTIFIER_RE.is_match(&name)
}
pub fn is_simple_identifier(name: &str) -> bool {
  !NON_IDENTIFIER_RE.is_match(name)
}

#[napi]
pub fn is_fn_expression(exp: SimpleExpressionNode) -> bool {
  let Some(mut ast) = exp.ast else {
    return false;
  };
  ast = unwrap_ts_node(ast);
  let Ok(_type) = ast.get_named_property::<String>("type") else {
    return false;
  };
  _type.eq("FunctionExpression") || _type.eq("ArrowFunctionExpression")
}

static FUNCTION_TYPE_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"Function(?:Expression|Declaration)$|Method$").unwrap());
/**
 * Checks if the given node is a function type.
 *
 * @param node - The node to check.
 * @returns True if the node is a function type, false otherwise.
 */
#[napi]
pub fn is_function_type(node: Object) -> bool {
  let Ok(node_type) = node.get_named_property::<String>("type") else {
    return false;
  };
  !node_type.starts_with("TS") && FUNCTION_TYPE_RE.is_match(&node_type)
}

#[napi]
pub fn is_identifier(node: Object) -> bool {
  let Ok(node_type) = node.get_named_property::<String>("type") else {
    return false;
  };
  node_type == "Identifier" || node_type == "JSXIdentifier"
}

#[napi]
pub fn is_static_property(node: Object) -> bool {
  let Ok(node_type) = node.get_named_property::<String>("type") else {
    return false;
  };
  node_type == "Property" && !node.get_named_property::<bool>("computed").unwrap_or(true)
}

#[napi]
pub fn is_for_statement(node: Object) -> bool {
  let Ok(node_type) = node.get_named_property::<String>("type") else {
    return false;
  };
  node_type == "ForOfStatement" || node_type == "ForInStatement" || node_type == "ForStatement"
}

// Checks if the input `node` is a reference to a bound variable.
//
// Copied from https://github.com/babel/babel/blob/main/packages/babel-types/src/validators/isReferenced.ts
//
// @param node - The node to check.
// @param parent - The parent node of the input `node`.
// @param grandparent - The grandparent node of the input `node`.
// @returns True if the input `node` is a reference to a bound variable, false otherwise.
#[napi]
pub fn is_referenced(
  env: Env,
  node: Object,
  parent: Object,
  grandparent: Option<Object>,
) -> Result<bool> {
  let parent_type = parent.get_named_property::<String>("type")?;
  Ok(match parent_type.as_str() {
    // yes: PARENT[NODE]
    // yes: NODE.child
    // no: parent.NODE
    "MemberExpression" => {
      if env.strict_equals(parent.get_named_property::<Object>("property")?, node)? {
        parent.get_named_property::<bool>("computed")?
      } else {
        env.strict_equals(parent.get_named_property::<Object>("object")?, node)?
      }
    }

    "JSXMemberExpression" => {
      env.strict_equals(parent.get_named_property::<Object>("object")?, node)?
    }

    // no: let NODE = init;
    // yes: let id = NODE;
    "VariableDeclarator" => {
      env.strict_equals(parent.get_named_property::<Object>("init")?, node)?
    }

    // yes: () => NODE
    // no: (NODE) => {}
    "ArrowFunctionExpression" => {
      env.strict_equals(parent.get_named_property::<Object>("body")?, node)?
    }

    // no: class { #NODE; }
    // no: class { get #NODE() {} }
    // no: class { #NODE() {} }
    // no: class { fn() { return this.#NODE; } }
    "PrivateIdentifier" => false,

    // no: class { NODE() {} }
    // yes: class { [NODE]() {} }
    // no: class { foo(NODE) {} }
    "MethodDefinition" => {
      if env.strict_equals(parent.get_named_property::<Object>("key")?, node)? {
        parent.get_named_property::<bool>("computed")?
      } else {
        false
      }
    }

    // yes: { [NODE]: "" }
    // no: { NODE: "" }
    // depends: { NODE }
    // depends: { key: NODE }
    //
    // no: class { NODE = value; }
    // yes: class { [NODE] = value; }
    // yes: class { key = NODE; }
    "Property" | "AccessorProperty" => {
      let key = parent.get_named_property::<Object>("key")?;
      if key
        .get_named_property::<String>("type")?
        .eq("PrivateIdentifier")
      {
        !env.strict_equals(parent.get_named_property::<Object>("key")?, node)?
      } else if env.strict_equals(key, node)? {
        parent.get_named_property::<bool>("computed")?
      }
      // parent.value === node
      else if let Some(grandparent) = grandparent {
        !grandparent
          .get_named_property::<String>("type")?
          .eq("ObjectPattern")
      } else {
        true
      }
    }

    // no: class NODE {}
    // yes: class Foo extends NODE {}
    "ClassDeclaration" | "ClassExpression" => {
      if let Ok(super_class) = parent.get_named_property::<Object>("superClass") {
        env.strict_equals(super_class, parent)?
      } else {
        false
      }
    }

    // yes: left = NODE;
    // no: NODE = right;
    //
    // no: [NODE = foo] = [];
    // yes: [foo = NODE] = [];
    "AssignmentExpression" | "AssignmentPattern" => {
      env.strict_equals(parent.get_named_property::<Object>("right")?, node)?
    }

    // no: NODE: for (;;) {}
    "LabeledStatement" => false,

    // no: try {} catch (NODE) {}
    "CatchClause" => false,

    // no: function foo(...NODE) {}
    "RestElement" => false,
    "BreakStatement" | "ContinueStatement" => false,

    // no: function NODE() {}
    // no: function foo(NODE) {}
    "FunctionDeclaration" | "FunctionExpression" => false,

    // no: export NODE from "foo";
    // no: export * as NODE from "foo";
    //
    // don't support in oxc
    // case 'ExportDefaultSpecifier':
    "ExportAllDeclaration" => false,

    // no: export { foo as NODE };
    // yes: export { NODE as foo };
    // no: export { NODE as foo } from "foo";
    "ExportSpecifier" => {
      if let Some(grandparent) = grandparent
        && grandparent
          .get_named_property::<String>("type")?
          .eq("ExportNamedDeclaration")
        && grandparent.get_named_property::<Object>("source").is_ok()
      {
        false
      } else {
        env.strict_equals(parent.get_named_property::<Object>("local")?, node)?
      }
    }

    // no: import NODE from "foo";
    // no: import * as NODE from "foo";
    // no: import { NODE as foo } from "foo";
    // no: import { foo as NODE } from "foo";
    // no: import NODE from "bar";
    "ImportDefaultSpecifier" | "ImportNamespaceSpecifier" | "ImportSpecifier" => false,

    // no: import "foo" assert { NODE: "json" }
    "ImportAttribute" => false,

    // no: <div NODE="foo" />
    // no: <div foo:NODE="foo" />
    "JSXAttribute" | "JSXNamespacedName" => false,

    // no: [NODE] = [];
    // no: ({ NODE }) = [];
    "ObjectPattern" | "ArrayPattern" => false,

    // no: new.NODE
    // no: NODE.target
    "MetaProperty" => false,

    // yes: enum X { Foo = NODE }
    // no: enum X { NODE }
    "TSEnumMember" => !env.strict_equals(parent.get_named_property::<Object>("id")?, node)?,

    // yes: { [NODE]: value }
    // no: { NODE: value }
    "TSPropertySignature" => {
      if env.strict_equals(parent.get_named_property::<Object>("key")?, node)? {
        parent.get_named_property::<bool>("computed")?
      } else {
        true
      }
    }
    _ => true,
  })
}

#[napi]
pub fn is_referenced_identifier(
  env: Env,
  id: Object,
  parent: Option<Object>,
  parent_stack: Vec<Object>,
) -> Result<bool> {
  let Some(parent) = parent else {
    return Ok(true);
  };

  // is a special keyword but parsed as identifier
  if id.get_named_property::<String>("name")?.eq("arguments") {
    return Ok(false);
  }

  if is_referenced(
    env,
    id,
    parent,
    if parent_stack.len() > 1 {
      Some(parent_stack[parent_stack.len() - 2])
    } else {
      None
    },
  )? {
    return Ok(true);
  }

  // babel's isReferenced check returns false for ids being assigned to, so we
  // need to cover those cases here
  Ok(
    match parent.get_named_property::<String>("type")?.as_str() {
      "AssignmentExpression" | "AssignmentPattern" => true,
      "Property" => {
        !env.strict_equals(parent.get_named_property::<Object>("key")?, id)?
          && is_in_destructure_assignment(Some(parent), parent_stack)
      }
      "ArrayPattern" => is_in_destructure_assignment(Some(parent), parent_stack),
      _ => false,
    },
  )
}

#[napi]
pub fn is_in_destructure_assignment(parent: Option<Object>, parent_stack: Vec<Object>) -> bool {
  let Some(parent) = parent else {
    return false;
  };
  let Ok(parent_type) = parent.get_named_property::<String>("type") else {
    return false;
  };
  if parent_type == "Property" || parent_type == "ArrayPattern" {
    let mut i = parent_stack.len();
    while i > 0 {
      i -= 1;
      let Ok(_type) = parent_stack[i].get_named_property::<String>("type") else {
        return false;
      };
      if _type == "AssignmentExpression" {
        return true;
      } else if _type != "Property" && !_type.ends_with("Pattern") {
        break;
      }
    }
  }
  return false;
}
