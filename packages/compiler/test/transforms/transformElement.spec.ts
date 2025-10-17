import { describe, expect, test } from 'vitest'
import {
  IRDynamicPropsKind,
  IRNodeTypes,
  transformChildren,
  transformElement,
  transformText,
  transformVBind,
  transformVFor,
  transformVOn,
} from '../../src'
import { makeCompile } from './_utils'

const compileWithElementTransform = makeCompile({
  nodeTransforms: [
    transformVFor,
    transformElement,
    transformText,
    transformChildren,
  ],
  directiveTransforms: {
    bind: transformVBind,
    on: transformVOn,
  },
})

describe('compiler: element transform', () => {
  describe('component', () => {
    test('import + resolve component', () => {
      const { code, ir, helpers } = compileWithElementTransform(`<Foo/>`, {
        withFallback: true,
      })
      expect(code).toMatchSnapshot()
      expect(helpers).contains.all.keys('resolveComponent')
      expect(helpers).contains.all.keys('createComponentWithFallback')
      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        id: 0,
        tag: 'Foo',
        asset: true,
        root: true,
        props: [[]],
      })
    })

    test('resolve namespaced component', () => {
      const { code, helpers } = compileWithElementTransform(`<Foo.Example/>`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`Foo.Example`)
      expect(helpers).not.toContain('resolveComponent')
    })

    test('generate single root component', () => {
      const { code } = compileWithElementTransform(`<Comp/>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp, null, null, true)')
    })

    test('generate multi root component', () => {
      const { code } = compileWithElementTransform(`<><Comp/>123</>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp)')
    })

    test('Fragment should not mark as single root', () => {
      const { code } = compileWithElementTransform(`<><Comp/></>`)
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp)')
    })

    test('v-for on component should not mark as single root', () => {
      const { code } = compileWithElementTransform(
        `<Comp v-for={item in items} key={item}/>`,
      )
      expect(code).toMatchSnapshot()
      expect(code).contains('_createComponent(Comp)')
    })

    test('static props', () => {
      const { code, ir } = compileWithElementTransform(
        `<Foo id="foo" class="bar" />`,
      )

      expect(code).toMatchSnapshot()
      expect(code).contains(`{
    id: () => ("foo"),
    class: () => ("bar")
  }`)

      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        tag: 'Foo',
        asset: false,
        root: true,
        props: [
          [
            {
              key: {
                content: 'id',
                isStatic: true,
              },
              values: [
                {
                  content: 'foo',
                  isStatic: true,
                },
              ],
            },
            {
              key: {
                content: 'class',
                isStatic: true,
              },
              values: [
                {
                  content: 'bar',
                  isStatic: true,
                },
              ],
            },
          ],
        ],
      })
    })

    test('{...obj}', () => {
      const { code, ir } = compileWithElementTransform(`<Foo {...obj} />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`[
    () => (obj)
  ]`)
      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        tag: 'Foo',
        props: [
          {
            kind: IRDynamicPropsKind.EXPRESSION,
            value: { content: 'obj', isStatic: false },
          },
        ],
      })
    })

    test('{...obj} after static prop', () => {
      const { code, ir } = compileWithElementTransform(
        `<Foo id="foo" {...obj} />`,
      )
      expect(code).toMatchSnapshot()
      expect(code).contains(`{
    id: () => ("foo"),
    $: [
      () => (obj)
    ]
  }`)
      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        tag: 'Foo',
        props: [
          [{ key: { content: 'id' }, values: [{ content: 'foo' }] }],
          {
            kind: IRDynamicPropsKind.EXPRESSION,
            value: { content: 'obj' },
          },
        ],
      })
    })

    test('{...obj} before static prop', () => {
      const { code, ir } = compileWithElementTransform(
        `<Foo {...obj} id="foo" />`,
      )
      expect(code).toMatchSnapshot()
      expect(code).contains(`[
    () => (obj),
    { id: () => ("foo") }
  ]`)
      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        tag: 'Foo',
        props: [
          {
            kind: IRDynamicPropsKind.EXPRESSION,
            value: { content: 'obj' },
          },
          [{ key: { content: 'id' }, values: [{ content: 'foo' }] }],
        ],
      })
    })

    test('{...obj} between static props', () => {
      const { code, ir } = compileWithElementTransform(
        `<Foo id="foo" {...obj} class="bar" />`,
      )
      expect(code).toMatchSnapshot()
      expect(code).contains(`{
    id: () => ("foo"),
    $: [
      () => (obj),
      { class: () => ("bar") }
    ]
  }`)
      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        tag: 'Foo',
        props: [
          [{ key: { content: 'id' }, values: [{ content: 'foo' }] }],
          {
            kind: IRDynamicPropsKind.EXPRESSION,
            value: { content: 'obj' },
          },
          [{ key: { content: 'class' }, values: [{ content: 'bar' }] }],
        ],
      })
    })

    test('props merging: style', () => {
      const { code } = compileWithElementTransform(
        `<Foo style="color: green" style={{ color: 'red' }} />`,
      )
      expect(code).toMatchSnapshot()
    })

    test('props merging: class', () => {
      const { code } = compileWithElementTransform(
        `<Foo class="foo" class={{ bar: isBar }} />`,
      )
      expect(code).toMatchSnapshot()
    })

    test('v-on={obj}', () => {
      const { code, ir } = compileWithElementTransform(`<Foo v-on={obj} />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(`[
    () => (_toHandlers(obj))
  ]`)
      expect(ir.block.dynamic.children[0].operation).toMatchObject({
        type: IRNodeTypes.CREATE_COMPONENT_NODE,
        tag: 'Foo',
        props: [
          {
            kind: IRDynamicPropsKind.EXPRESSION,
            value: { content: 'obj' },
            handler: true,
          },
        ],
      })
    })

    test('component event with once modifier', () => {
      const { code } = compileWithElementTransform(`<Foo onFoo_once={bar} />`)
      expect(code).toMatchSnapshot()
      expect(code).includes('onFooOnce: () => bar')
    })

    test('component with fallback', () => {
      const { code } = compileWithElementTransform(`<foo-bar />`)
      expect(code).toMatchSnapshot()
      expect(code).contains(
        '_createComponentWithFallback(_component_foo_bar, null, null, true)',
      )
    })
  })

  test('static props', () => {
    const { code, ir } = compileWithElementTransform(
      `<div id="foo" class="bar" />`,
    )

    const template = '<div id="foo" class="bar"></div>'
    expect(code).toMatchSnapshot()
    expect(ir.templates).toMatchObject([template])
    expect(ir.block.effect).lengthOf(0)
  })

  test('props + children', () => {
    const { code, ir } = compileWithElementTransform(
      `<div id="foo"><span/></div>`,
    )

    const template = '<div id="foo"><span></span></div>'
    expect(code).toMatchSnapshot()
    expect(ir.templates).toMatchObject([template])
    expect(ir.block.effect).lengthOf(0)
  })

  test('{...obj}', () => {
    const { code, ir } = compileWithElementTransform(`<div {...obj} />`)
    expect(code).toMatchSnapshot()
    expect(ir.block.effect).toMatchObject([
      {
        expressions: [],
        operations: [
          {
            type: IRNodeTypes.SET_DYNAMIC_PROPS,
            element: 0,
            props: [
              {
                kind: IRDynamicPropsKind.EXPRESSION,
                value: {
                  content: 'obj',
                  isStatic: false,
                },
              },
            ],
          },
        ],
      },
    ])
    expect(code).contains('_setDynamicProps(n0, [obj], true)')
  })

  test('{...obj} after static prop', () => {
    const { code, ir } = compileWithElementTransform(
      `<div id="foo" {...obj} />`,
    )
    expect(code).toMatchSnapshot()
    expect(ir.block.effect).toMatchObject([
      {
        expressions: [],
        operations: [
          {
            type: IRNodeTypes.SET_DYNAMIC_PROPS,
            element: 0,
            props: [
              [{ key: { content: 'id' }, values: [{ content: 'foo' }] }],
              {
                kind: IRDynamicPropsKind.EXPRESSION,
                value: {
                  content: 'obj',
                  isStatic: false,
                },
              },
            ],
          },
        ],
      },
    ])
    expect(code).contains('_setDynamicProps(n0, [{ id: "foo" }, obj], true)')
  })

  test('{...obj} before static prop', () => {
    const { code, ir } = compileWithElementTransform(
      `<div {...obj} id="foo" />`,
    )
    expect(code).toMatchSnapshot()
    expect(ir.block.effect).toMatchObject([
      {
        expressions: [],
        operations: [
          {
            type: IRNodeTypes.SET_DYNAMIC_PROPS,
            element: 0,
            props: [
              {
                kind: IRDynamicPropsKind.EXPRESSION,
                value: { content: 'obj' },
              },
              [{ key: { content: 'id' }, values: [{ content: 'foo' }] }],
            ],
          },
        ],
      },
    ])
    expect(code).contains('_setDynamicProps(n0, [obj, { id: "foo" }], true)')
  })

  test('{...obj} between static props', () => {
    const { code, ir } = compileWithElementTransform(
      `<div id="foo" {...obj} class="bar" />`,
    )
    expect(code).toMatchSnapshot()
    expect(ir.block.effect).toMatchObject([
      {
        expressions: [],
        operations: [
          {
            type: IRNodeTypes.SET_DYNAMIC_PROPS,
            element: 0,
            props: [
              [{ key: { content: 'id' }, values: [{ content: 'foo' }] }],
              {
                kind: IRDynamicPropsKind.EXPRESSION,
                value: { content: 'obj' },
              },
              [{ key: { content: 'class' }, values: [{ content: 'bar' }] }],
            ],
          },
        ],
      },
    ])
    expect(code).contains(
      '_setDynamicProps(n0, [{ id: "foo" }, obj, { class: "bar" }], true)',
    )
  })

  test('props merging: event handlers', () => {
    const { code, ir } = compileWithElementTransform(
      `<div onClick_foo={a} onClick_bar={b} />`,
    )
    expect(code).toMatchSnapshot()

    expect(ir.block.operation).toMatchObject([
      {
        type: IRNodeTypes.SET_EVENT,
        element: 0,
        key: {
          content: 'click',
          isStatic: true,
        },
        value: {
          content: 'a',
          isStatic: false,
        },
        delegate: true,
        effect: false,
      },
      {
        type: IRNodeTypes.SET_EVENT,
        element: 0,
        key: {
          content: 'click',
          isStatic: true,
        },
        value: {
          content: 'b',
          isStatic: false,
        },
        delegate: true,
        effect: false,
      },
    ])
  })

  test('props merging: style', () => {
    const { code, ir } = compileWithElementTransform(
      `<div style="color: green" style={{ color: 'red' }} />`,
    )
    expect(code).toMatchSnapshot()

    expect(ir.block.operation).toMatchObject([
      {
        type: IRNodeTypes.SET_PROP,
        element: 0,
        prop: {
          key: {
            content: 'style',
            isStatic: true,
          },
          values: [
            {
              content: 'color: green',
              isStatic: true,
            },
            {
              content: `{ color: 'red' }`,
              isStatic: false,
            },
          ],
        },
      },
    ])
  })

  test('props merging: class', () => {
    const { code, ir } = compileWithElementTransform(
      `<div class="foo" class={{ bar: isBar }} />`,
    )

    expect(code).toMatchSnapshot()

    expect(ir.block.effect).toMatchObject([
      {
        expressions: [],
        operations: [
          {
            type: IRNodeTypes.SET_PROP,
            element: 0,
            prop: {
              key: {
                content: 'class',
                isStatic: true,
              },
              values: [
                {
                  content: `foo`,
                  isStatic: true,
                },
                {
                  content: `{ bar: isBar }`,
                  isStatic: false,
                },
              ],
            },
          },
        ],
      },
    ])
  })

  test('v-on="obj"', () => {
    const { code, ir } = compileWithElementTransform(`<div v-on={obj} />`)
    expect(code).toMatchSnapshot()
    expect(ir.block.effect).toMatchObject([
      {
        expressions: [],
        operations: [
          {
            type: IRNodeTypes.SET_DYNAMIC_EVENTS,
            element: 0,
            value: {
              content: 'obj',
              isStatic: false,
            },
          },
        ],
      },
    ])
    expect(code).contains('_setDynamicEvents(n0, obj)')
  })

  test('invalid html nesting', () => {
    const { code, ir } = compileWithElementTransform(
      `<><p><div>123</div></p>
      <form><form/></form></>`,
    )
    expect(code).toMatchSnapshot()
    expect(ir.templates).toEqual(['<div>123</div>', '<p></p>', '<form></form>'])
    expect(ir.block.dynamic).toMatchObject({
      children: [
        {
          id: 1,
          template: 1,
          flags: 1,
          hasDynamicChild: true,
          children: [
            {
              id: 0,
              template: 0,
              flags: 7,
              children: [{ children: [], flags: 1 }],
            },
          ],
        },
        {
          children: [],
          flags: 3,
          id: 2,
        },
        { id: 4, template: 2, children: [{ id: 3, template: 2 }] },
      ],
    })

    expect(ir.block.operation).toMatchObject([
      { type: IRNodeTypes.INSERT_NODE, parent: 1, elements: [0] },
      { type: IRNodeTypes.INSERT_NODE, parent: 4, elements: [3] },
    ])
  })

  test('number value', () => {
    const { code } = compileWithElementTransform(`<div foo={1} />`)
    expect(code).toMatchSnapshot()
    expect(code).not.contains('_setProp(n0, "foo", 1)')
  })
})
