import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: template ref transform', () => {
  test('static ref', () => {
    const { code, templates } = compile(`<div ref="foo" />`)

    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          true,
        ],
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
        [
          "<div></div>",
          true,
        ],
      ]
    `)
    expect(code).matchSnapshot()
    expect(code).contains('_setTemplateRef(n0, foo, r0)')
  })

  test('function ref', () => {
    const { code, templates } = compile(
      `<Comp v-slot={{baz}}>
          <div ref={bar => {
            foo.value = bar
            ;({ baz, bar: baz } = bar)
            console.log(foo.value, baz)
          }} />
      </Comp>`,
    )
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
      ]
    `)
    expect(code).toMatchSnapshot()
    expect(code).contains('const _setTemplateRef = _createTemplateRefSetter()')
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
