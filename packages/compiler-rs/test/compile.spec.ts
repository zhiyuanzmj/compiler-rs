import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compile', () => {
  test('static template', () => {
    const { code } = compile(
      `<div>
        <div>hello</div>
        <input />
        <span />
      </div>`,
    )
    expect(code).toMatchSnapshot()
  })

  test('dynamic root', () => {
    const { code } = compile(`<>{ 1 }{ 2 }</>`)
    expect(code).toMatchSnapshot()
  })

  test('dynamic root', () => {
    const { code } = compile(`<div>{a +b +       c }</div>`)
    expect(code).toMatchSnapshot()
  })

  describe('expression parsing', () => {
    test('interpolation', () => {
      const { code } = compile(`<>{ a + b }</>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('a + b')
    })
  })

  describe('setInsertionState', () => {
    test('next, child and nthChild should be above the setInsertionState', () => {
      const { code } = compile(`
      <div>
        <div />
        <Comp />
        <div />
        <div v-if={true} />
        <div>
          <button disabled={foo} />
        </div>
      </div>
      `)
      expect(code).toMatchSnapshot()
    })
  })

  describe('execution order', () => {
    test('basic', () => {
      const { code } = compile(`<div foo={true}>{foo}</div>`)
      expect(code).matchSnapshot()
      expect(code).contains(
        `_setProp(n0, "foo", true)
  const x0 = _child(n0)
  _setNodes(x0, () => (foo))`,
      )
    })
    test('with v-once', () => {
      const { code } = compile(
        `<div>
          <span v-once>{ foo }</span>
          { bar }<br/>
          { baz }
        </div>`,
      )
      expect(code).matchSnapshot()
      expect(code).contains(
        `_setNodes(n1, () => (bar))
  _setNodes(n2, () => (baz))`,
      )
    })
  })
})

describe('directive', () => {
  describe('custom directive', () => {
    test('basic', () => {
      const { code } = compile(`<div v-example></div>`, { withFallback: true })
      expect(code).matchSnapshot()
    })

    test('binding value', () => {
      const { code } = compile(`<div v-example={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('static parameters', () => {
      const { code } = compile(`<div v-example:foo={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('modifiers', () => {
      const { code } = compile(`<div v-example_bar={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('modifiers w/o binding', () => {
      const { code } = compile(`<div v-example_foo-bar></div>`)
      expect(code).matchSnapshot()
    })

    test('static parameters and modifiers', () => {
      const { code } = compile(`<div v-example:foo_bar={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('dynamic parameters', () => {
      const { code } = compile(`<div v-example:$foo$={msg}></div>`)
      expect(code).matchSnapshot()
    })

    test('component', () => {
      const { code } = compile(`
      <Comp v-test>
        <div v-if="true">
          <Bar v-hello_world />
        </div>
      </Comp>
      `)
      expect(code).matchSnapshot()
    })
  })
})
