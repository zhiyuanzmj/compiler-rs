import { compile, ErrorCodes } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test, vi } from 'vitest'

describe('compiler: transform slot', () => {
  test('implicit default slot', () => {
    const { code, templates } = compile(`<Comp><div/></Comp>`)
    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
      ]
    `)
  })

  test('on-component default slot', () => {
    const { code } = compile(`<Comp v-slot={scope}>{ scope.foo + bar }</Comp>`)
    expect(code).toMatchSnapshot()

    expect(code).contains(`default: (scope) =>`)
    expect(code).contains(`scope.foo + bar`)
  })

  test('on component named slot', () => {
    const { code } = compile(`<Comp v-slot:named={({ foo })}>{{ foo }}</Comp>`)
    expect(code).toMatchSnapshot()

    expect(code).contains(`named: (_slotProps0) =>`)
    expect(code).contains(`{ foo: _slotProps0.foo }`)
  })

  test('on component dynamically named slot', () => {
    const { code } = compile(`<Comp v-slot:$named$={{ foo }}>{ foo + bar }</Comp>`)
    expect(code).toMatchSnapshot()

    expect(code).contains(`fn: (_slotProps0) =>`)
    expect(code).contains(`_slotProps0.foo + bar`)
  })

  test('named slots w/ implicit default slot', () => {
    const { templates, code } = compile(
      `<Comp>
        <template v-slot:one>foo</template>bar<span/>
      </Comp>`,
    )
    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "foo",
          false,
        ],
        [
          "bar",
          false,
        ],
        [
          "<span></span>",
          false,
        ],
      ]
    `)
  })

  test('named slots w/ comment', () => {
    const { code } = compile(
      `<Comp>
        {/* foo */}
        <template v-slot:one>foo</template>
      </Comp>`,
    )
    expect(code).toMatchSnapshot()
  })

  test('nested slots scoping', () => {
    const { code } = compile(
      `<Comp>
        <template v-slot:default={{ foo }}>
          <Inner v-slot={{ bar }}>
            { foo + bar + baz }
          </Inner>
          { foo + bar + baz }
        </template>
      </Comp>`,
    )
    expect(code).toMatchSnapshot()

    expect(code).contains(`default: (_slotProps0) =>`)
    expect(code).contains(`default: (_slotProps1) =>`)
    expect(code).contains(`_slotProps0.foo + _slotProps1.bar + baz`)
    expect(code).contains(`_slotProps0.foo + bar + baz`)
  })

  test('dynamic slots name', () => {
    const { code } = compile(
      `<Comp>
        <template v-slot:$name$>{foo}</template>
      </Comp>`,
    )
    expect(code).toMatchSnapshot()
  })

  test('dynamic slots name w/ v-for', () => {
    const { code } = compile(
      `<Comp>
        <template v-for={item in list} v-slot:$item$={{ bar }}>{ bar }</template>
      </Comp>`,
    )
    expect(code).toMatchSnapshot()

    expect(code).contains(`fn: (_slotProps0) =>`)
    expect(code).contains(`_createNodes(() => _slotProps0.bar)`)
  })

  test('dynamic slots name w/ v-if / v-else[-if]', () => {
    const { code } = compile(
      `<Comp>
        <template v-if={condition} v-slot:condition>condition slot</template>
        <template v-else-if={anotherCondition} v-slot:condition={{ foo, bar }}>another condition</template>
        <template v-else-if={otherCondition} v-slot:condition>other condition</template>
        <template v-else v-slot:condition>else condition</template>
      </Comp>`,
    )
    expect(code).toMatchSnapshot()

    expect(code).contains(`fn: (_slotProps0) =>`)
  })

  test('quote slot name', () => {
    const { code } = compile(`<Comp><template v-slot:nav-bar-title-before></template></Comp>`)
    expect(code).toMatchSnapshot()
    expect(code).contains(`"nav-bar-title-before"`)
  })

  test('nested component slot', () => {
    const { code } = compile(`<A><B/></A>`)
    expect(code).toMatchSnapshot()
  })

  describe('errors', () => {
    test('error on extraneous children w/ named default slot', () => {
      const onError = vi.fn()
      const source = `<Comp><template v-slot:default>foo</template>bar</Comp>`
      compile(source, { onError })
      // const index = source.indexOf('bar')
      expect(onError.mock.calls[0][0]).toMatchObject({
        code: ErrorCodes.VSlotExtraneousDefaultSlotChildren,
        // loc: {
        //   start: {
        //     index,
        //     line: 1,
        //     column: index,
        //   },
        //   end: {
        //     index: index + 3,
        //     line: 1,
        //     column: index + 3,
        //   },
        // },
      })
    })

    test('error on duplicated slot names', () => {
      const onError = vi.fn()
      const source = `<Comp><template v-slot:foo></template><template v-slot:foo></template></Comp>`
      compile(source, { onError })
      // const index = source.lastIndexOf('v-slot:foo')
      expect(onError.mock.calls[0][0]).toMatchObject({
        code: ErrorCodes.VSlotDuplicateSlotNames,
        // loc: {
        //   start: {
        //     index,
        //     line: 1,
        //     column: index,
        //   },
        //   end: {
        //     index: index + 10,
        //     line: 1,
        //     column: index + 10,
        //   },
        // },
      })
    })

    test('error on invalid mixed slot usage', () => {
      const onError = vi.fn()
      const source = `<Comp v-slot={foo}><template v-slot:foo></template></Comp>`
      compile(source, { onError })
      // const index = source.lastIndexOf('v-slot={foo}')
      expect(onError.mock.calls[0][0]).toMatchObject({
        code: ErrorCodes.VSlotMixedSlotUsage,
        // loc: {
        //   start: {
        //     index,
        //     line: 1,
        //     column: index,
        //   },
        //   end: {
        //     index: index + 12,
        //     line: 1,
        //     column: index + 12,
        //   },
        // },
      })
    })

    test('error on v-slot usage on plain elements', () => {
      const onError = vi.fn()
      const source = `<div v-slot/>`
      compile(source, { onError })
      // const index = source.indexOf('v-slot')
      expect(onError.mock.calls[0][0]).toMatchObject({
        code: ErrorCodes.VSlotMisplaced,
        // loc: {
        //   start: {
        //     index,
        //     line: 1,
        //     column: index,
        //   },
        //   end: {
        //     index: index + 6,
        //     line: 1,
        //     column: index + 6,
        //   },
        // },
      })
    })
  })
})
