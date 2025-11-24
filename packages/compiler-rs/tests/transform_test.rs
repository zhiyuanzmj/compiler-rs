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
      interop: true,
      ..Default::default()
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
