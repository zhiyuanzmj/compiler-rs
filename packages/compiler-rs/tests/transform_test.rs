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
  let code = transform(source, Some(TransformOptions::build(source, vec![], true))).code;
  assert_snapshot!(code);
}
