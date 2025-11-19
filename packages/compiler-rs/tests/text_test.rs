use compiler_rs::compile::compile;
use insta::assert_snapshot;

#[test]
fn text_test() {
  let result = compile(
    "<div>
        {count.value === 1 ? (
        <div>{count.value}</div>
        ) : count.value === 2 ? (
        <Foo />
        ) : count.value >= 3 ? (
        <div>lg 3: {count.value}</div>
        ) : (
        <div>lt 0: {count.value}</div>
        )}
    </div>",
    None,
  );
  assert_snapshot!(result.code);
}
