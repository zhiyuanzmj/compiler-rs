use std::{cell::RefCell, collections::HashSet};

use compiler_rs::transform::{TransformOptions, transform};
use insta::assert_snapshot;

#[test]
fn transform_test() {
  let source = "const A = defineComponent(() => {
       defineVaporComponent(() => <span />)
       return () => <div />
     })
     const B = defineVaporComponent(() => {
      const C = defineComponent(() => <div />)
      const D = <>{foo} <div /></>
      return <div />
     })";
  let code = transform(
    source,
    Some(TransformOptions {
      filename: "index.tsx",
      templates: RefCell::new(vec![]),
      helpers: RefCell::new(HashSet::new()),
      delegates: RefCell::new(HashSet::new()),
      source_map: false,
      with_fallback: false,
      is_custom_element: Box::new(|_| false),
      on_error: Box::new(|_| {}),
      interop: true,
    }),
  )
  .code;
  assert_snapshot!(code);
}

#[test]
fn map_expression_test() {
  let source = "<>{Array.from({ length: count.value }).map((_, index) => {
      if (index > 1) {
        return <div>1</div>
      } else {
        return [<span>({index}) lt 1</span>, <br />]
      }
    })}</>";
  let code = transform(source, None).code;
  assert_snapshot!(code);
}
