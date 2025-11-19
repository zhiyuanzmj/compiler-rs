import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: element transform', () => {
  describe('component', () => {
    test('import + resolve component', () => {
      const { code, helpers } = compile(`<Foo/>`, {
        withFallback: true,
      })
      expect(code).toMatchSnapshot()
      expect(helpers).contains.all.keys('resolveComponent')
      expect(helpers).contains.all.keys('createComponentWithFallback')
    })

    test('resolve namespaced component', () => {
      const { code, helpers } = compile(`<Foo.Example/>`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`Foo.Example`)
      expect(helpers).not.toContain('resolveComponent')
    })

    test('generate single root component', () => {
      const { code } = compile(`<Comp/>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp, null, null, true)')
    })

    test('generate multi root component', () => {
      const { code } = compile(`<><Comp/>123</>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp)')
    })

    test('Fragment should not mark as single root', () => {
      const { code } = compile(`<><Comp/></>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp)')
    })

    test('v-for on component should not mark as single root', () => {
      const { code } = compile(`<Comp v-for={item in items} key={item}/>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp)')
    })

    test('static props', () => {
      const { code } = compile(`<Foo id="foo" class="bar" />`)

      expect(code).toMatchSnapshot()
      expect(code).contains(`{
    id: () => "foo",
    class: () => "bar"
  }`)
    })

    test('{...obj}', () => {
      const { code } = compile(`<Foo {...obj} />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`[() => obj]`)
    })

    test('{...obj} after static prop', () => {
      const { code } = compile(`<Foo id="foo" {...obj} />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`{
    id: () => "foo",
    $: [() => obj]
  }`)
    })

    test('{...obj} before static prop', () => {
      const { code } = compile(`<Foo {...obj} id="foo" />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`[() => obj, { id: () => "foo" }]`)
    })

    test('{...obj} between static props', () => {
      const { code } = compile(`<Foo id="foo" {...obj} class="bar" />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`{
    id: () => "foo",
    $: [() => obj, { class: () => "bar" }]
  }`)
    })

    test('props merging: style', () => {
      const { code } = compile(`<Foo style="color: green" style={{ color: 'red' }} />`)
      expect(code).toMatchSnapshot()
    })

    test('props merging: class', () => {
      const { code } = compile(`<Foo class="foo" class={{ bar: isBar }} />`)
      expect(code).toMatchSnapshot()
    })

    test('v-on={obj}', () => {
      const { code } = compile(`<Foo v-on={obj} />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`[() => _toHandlers(obj)]`)
    })

    test('component event with once modifier', () => {
      const { code } = compile(`<Foo onFoo_once={bar} />`)
      expect(code).toMatchSnapshot()
      expect(code).includes('onFooOnce: () => bar')
    })

    test('component with fallback', () => {
      const { code } = compile(`<foo-bar />`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponentWithFallback(_component_foo_bar, null, null, true)')
    })
  })

  test('static props', () => {
    const { code, templates } = compile(`<div id="foo" class="bar" />`)

    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div id="foo" class="bar"></div>",
          true,
        ],
      ]
    `)
  })

  test('props + children', () => {
    const { code, templates } = compile(`<div id="foo"><span/></div>`)

    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div id="foo"><span></span></div>",
          true,
        ],
      ]
    `)
  })

  test('{...obj}', () => {
    const { code, templates } = compile(`<div {...obj} />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_setDynamicProps(n0, [obj], true)')
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          true,
        ],
      ]
    `)
  })

  test('{...obj} after static prop', () => {
    const { code } = compile(`<div id="foo" {...obj} />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_setDynamicProps(n0, [{ id: "foo" }, obj], true)')
  })

  test('{...obj} before static prop', () => {
    const { code } = compile(`<div {...obj} id="foo" />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_setDynamicProps(n0, [obj, { id: "foo" }], true)')
  })

  test('{...obj} between static props', () => {
    const { code } = compile(`<div id="foo" {...obj} class="bar" />`)
    expect(code).toMatchSnapshot()
    expect(code).contains(`_setDynamicProps(n0, [
    { id: "foo" },
    obj,
    { class: "bar" }
  ], true)`)
  })

  test('props merging: event handlers', () => {
    const { code } = compile(`<div onClick_foo={a} onClick_bar={b} />`)
    expect(code).toMatchSnapshot()
  })

  test('props merging: style', () => {
    const { code } = compile(`<div style="color: green" style={{ color: 'red' }} />`)
    expect(code).toMatchSnapshot()
  })

  test('props merging: class', () => {
    const { code } = compile(`<div class="foo" class={{ bar: isBar }} />`)

    expect(code).toMatchSnapshot()
  })

  test('v-on="obj"', () => {
    const { code } = compile(`<div v-on={obj} />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_setDynamicEvents(n0, obj)')
  })

  test('invalid html nesting', () => {
    const { code, templates } = compile(
      `<><p><div>123</div></p>
      <form><form/></form></>`,
    )
    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div>123</div>",
          false,
        ],
        [
          "<p></p>",
          false,
        ],
        [
          "<form></form>",
          false,
        ],
      ]
    `)
  })

  test('number value', () => {
    const { code } = compile(`<div foo={1} />`)
    expect(code).toMatchSnapshot()
    expect(code).not.contains('_setProp(n0, "foo", 1)')
  })
})
