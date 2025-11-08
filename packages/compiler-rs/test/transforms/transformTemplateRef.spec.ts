import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: template ref transform', () => {
  test('static ref', () => {
    const { code, templates } = compile(`<div ref="foo" />`)

    expect(templates).toMatchInlineSnapshot(`
      [
        "_template("<div></div>", true)",
      ]
    `)
    expect(code).matchSnapshot()
    expect(code).contains('const _setTemplateRef = _createTemplateRefSetter()')
    expect(code).contains('_setTemplateRef(n0, "foo")')
  })

  test('dynamic ref', () => {
    const { code, templates } = compile(`<div ref={foo} />`)

    expect(templates).toMatchInlineSnapshot(`
      [
        "_template("<div></div>", true)",
      ]
    `)
    expect(code).matchSnapshot()
    expect(code).contains('_setTemplateRef(n0, foo, r0)')
  })

  test('function ref', () => {
    const { code, templates } = compile(
      `<div ref={bar => {
        foo.value = bar
        ;({ baz } = bar)
        console.log(foo.value, baz)
      }} />`,
    )
    expect(templates).toMatchInlineSnapshot(`
      [
        "_template("<div></div>", true)",
      ]
    `)
    expect(code).toMatchSnapshot()
    expect(code).contains('const _setTemplateRef = _createTemplateRefSetter()')
    expect(code).contains(`_setTemplateRef(n0, bar => {
        foo.value = bar
        ;({ baz: baz } = bar)
        console.log(foo.value, baz)
      }, r0)`)
  })

  test('ref + v-if', () => {
    const { code } = compile(`<div ref={foo} v-if={true} />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_setTemplateRef(n2, foo, r2)')
  })

  test('ref + v-for', () => {
    const { code } = compile(`<div ref={foo} v-for={item in [1,2,3]} />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_setTemplateRef(n2, foo, r2, true)')
  })
})
