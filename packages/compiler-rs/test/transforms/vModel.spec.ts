import { compile, ErrorCodes } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test, vi } from 'vitest'

describe('compiler: vModel transform', () => {
  test('should support simple expression', () => {
    const { code, helpers } = compile('<input v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applyTextModel')
  })

  describe('modifiers', () => {
    test('.number', () => {
      const { code } = compile('<input v-model_number={model} />')

      expect(code).toMatchSnapshot()
    })

    test('.trim', () => {
      const { code } = compile('<input v-model_trim={model} />')

      expect(code).toMatchSnapshot()
    })

    test('.lazy', () => {
      const { code } = compile('<input v-model_lazy={model} />')

      expect(code).toMatchSnapshot()
    })
  })

  test('should support input (text)', () => {
    const { code, helpers } = compile('<input type="text" v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applyTextModel')
  })

  test('should support input (radio)', () => {
    const { code, helpers } = compile('<input type="radio" v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applyRadioModel')
  })

  test('should support input (checkbox)', () => {
    const { code, helpers } = compile('<input type="checkbox" v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applyCheckboxModel')
  })

  test('should support select', () => {
    const { code, helpers } = compile('<select v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applySelectModel')
  })

  test('should support textarea', () => {
    const { code, helpers } = compile('<textarea v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applyTextModel')
  })

  test('should support input (dynamic type)', () => {
    const { code, helpers } = compile('<input type={foo} v-model={model} />')
    expect(code).toMatchSnapshot()
    expect(helpers).toContain('applyDynamicModel')
  })

  test('should support w/ dynamic v-bind', () => {
    const root = compile('<input {...obj} v-model={model} />')
    expect(root.code).toMatchSnapshot()
    expect(root.helpers).toContain('applyDynamicModel')
  })

  describe('errors', () => {
    test('invalid element', () => {
      const onError = vi.fn()
      compile('<span v-model={model} />', { onError })

      expect(onError).toHaveBeenCalledTimes(1)
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: ErrorCodes.VModelOnInvalidElement,
        }),
      )
    })

    test('plain elements with argument', () => {
      const onError = vi.fn()
      compile('<input v-model:value={model} />', { onError })

      expect(onError).toHaveBeenCalledTimes(1)
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: ErrorCodes.VModelArgOnElement,
        }),
      )
    })

    // TODO: component
    test.fails('should allow usage on custom element', () => {
      const onError = vi.fn()
      const root = compile('<my-input v-model={model} />', {
        onError,
        isCustomElement: (tag: any) => tag.startsWith('my-'),
      })
      expect(root.helpers).toContain('vModelText')
      expect(onError).not.toHaveBeenCalled()
    })

    test('should raise error if used file input element', () => {
      const onError = vi.fn()
      compile(`<input type="file" v-model={test} />`, {
        onError,
      })
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: ErrorCodes.VModelOnFileInputElement,
        }),
      )
    })

    test('should error on dynamic value binding alongside v-model', () => {
      const onError = vi.fn()
      compile(`<input v-model={test} value={test} />`, {
        onError,
      })
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: ErrorCodes.VModelUnnecessaryValue,
        }),
      )
    })

    // #3596
    test('should NOT error on static value binding alongside v-model', () => {
      const onError = vi.fn()
      compile(`<input v-model={test} value="test" />`, {
        onError,
      })
      expect(onError).not.toHaveBeenCalled()
    })

    test('empty expression', () => {
      const onError = vi.fn()
      compile('<span v-model="" />', { onError })

      expect(onError).toHaveBeenCalledTimes(1)
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: ErrorCodes.VModelMalformedExpression,
        }),
      )
    })

    test('mal-formed expression', () => {
      const onError = vi.fn()
      compile('<span v-model={a + b} />', { onError })

      expect(onError).toHaveBeenCalledTimes(1)
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          code: ErrorCodes.VModelMalformedExpression,
        }),
      )
    })
  })

  test('should support member expression', () => {
    const { code } = compile('<input v-model={setupRef.child} />')

    expect(code).toMatchSnapshot()
  })

  test('should support member expression w/ inline', () => {
    const { code } = compile(
      '<><input v-model={setupRef.child} /><input v-model={setupLet.child} /><input v-model={setupMaybeRef.child} /></>',
    )

    expect(code).toMatchSnapshot()
  })

  describe('component', () => {
    test('v-model for component should work', () => {
      const { code } = compile('<Comp v-model={foo} />')
      expect(code).toMatchSnapshot()
      expect(code).contains(`modelValue: () => (foo),`)
      expect(code).contains(`"onUpdate:modelValue": () => _value => (foo = _value)`)
    })

    test('v-model with arguments for component should work', () => {
      const { code } = compile('<Comp v-model:bar={foo} />')
      expect(code).toMatchSnapshot()
      expect(code).contains(`bar: () => (foo),`)
      expect(code).contains(`"onUpdate:bar": () => _value => (foo = _value)`)
    })

    test('v-model with dynamic arguments for component should work', () => {
      const { code } = compile('<Comp v-model:$arg$={foo} />')
      expect(code).toMatchSnapshot()
      expect(code).contains(`[arg]: foo,`)
      expect(code).contains(`["onUpdate:" + arg]: () => _value => (foo = _value)`)
    })

    test('v-model with dynamic arguments for component w/ v-for', () => {
      const { code } = compile('<Comp v-for={{arg} in list} v-model:$arg$={foo} />')
      expect(code).toMatchSnapshot()
      expect(code).contains(`[_for_item0.value.arg]: foo,`)
      expect(code).contains(`["onUpdate:" + _for_item0.value.arg]: () => _value => (foo = _value)`)
    })

    test('v-model for component should generate modelValueModifiers', () => {
      const { code } = compile('<Comp v-model_trim_bar-baz={foo} />')
      expect(code).toMatchSnapshot()
      expect(code).contain(`modelValueModifiers: () => ({ trim: true, "bar-baz": true })`)
    })

    test('v-model with arguments for component should generate modelModifiers', () => {
      const { code } = compile('<Comp v-model:foo_trim={foo} v-model:bar_number={bar} />')
      expect(code).toMatchSnapshot()
      expect(code).contain(`fooModifiers: () => ({ trim: true })`)
      expect(code).contain(`barModifiers: () => ({ number: true })`)
    })

    test('v-model with dynamic arguments for component should generate modelModifiers ', () => {
      const { code } = compile('<Comp v-model:$foo$_trim={foo} v-model:$bar_value$_number={bar} />')
      expect(code).toMatchSnapshot()
      expect(code).contain(`[foo + "Modifiers"]: () => ({ trim: true })`)
      expect(code).contain(`[bar.value + "Modifiers"]: () => ({ number: true })`)
    })
  })
})
