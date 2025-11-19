use compiler_rs::compile::compile;
use insta::assert_snapshot;

#[test]
fn v_for_test() {
  let result = compile(
    "<tr
      v-for={row in rows}
      key={row.id}
      v-text={selected === row.id ? 'danger' : ''}
    ></tr>",
    None,
  );
  assert_snapshot!(result.code);
}
