import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: children transform', () => {
  test('basic', () => {
    const { code, helpers } = compile(
      `<div>
        {foo} {bar}
       </div>`,
    )
    expect(code).toMatchSnapshot()
    expect(helpers).contains.all.keys('setNodes')
  })

  test('comments', () => {
    const { code } = compile('<>{/*foo*/}<div>{/*bar*/}</div></>')
    expect(code).toMatchSnapshot()
  })

  test('fragment', () => {
    const { code } = compile('<>{foo}</>')
    expect(code).toMatchSnapshot()
  })

  test('children & sibling references', () => {
    const { code, helpers } = compile(
      `<div id={id}>
        <p>{ first }</p>
        123 { second } 456 {foo}
        <p>{ forth }</p>
      </div>`,
    )
    expect(code).toMatchSnapshot()
    expect(Array.from(helpers)).containSubset(['child', 'renderEffect', 'next', 'setNodes', 'template'])
  })

  test('efficient traversal', () => {
    const { code } = compile(
      `<div>
    <div>x</div>
    <div><span>{{ msg }}</span></div>
    <div><span>{{ msg }}</span></div>
    <div><span>{{ msg }}</span></div>
  </div>`,
    )
    expect(code).toMatchSnapshot()
  })

  test('efficient find', () => {
    const { code } = compile(
      `<div>
        <div>x</div>
        <div>x</div>
        <div>{ msg }</div>
      </div>`,
    )
    expect(code).contains(`const n0 = _nthChild(n1, 2)`)
    expect(code).toMatchSnapshot()
  })

  test('anchor insertion in middle', () => {
    const { code, templates } = compile(
      `<div>
        <div></div>
        <div v-if={1}></div>
        <div></div>
      </div>`,
    )
    // ensure the insertion anchor is generated before the insertion statement
    expect(code).toMatch(`const n3 = _next(_child(n4))`)
    expect(code).toMatch(`_setInsertionState(n4, n3)`)
    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
        [
          "<div><div></div><!><div></div></div>",
          true,
        ],
      ]
    `)
  })

  test('JSXComponent in JSXExpressionContainer', () => {
    const { code } = compile(
      `<div>
        {<Comp />}
      </div>`,
    )
    expect(code).toMatchSnapshot()
    expect(code).contains(`_setNodes(x0, () => (() => {
    const n0 = _createComponent(Comp, null, null, true);
    return n0;
  })())`)
  })
})
