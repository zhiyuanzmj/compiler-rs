import { compile, ErrorCodes } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test, vi } from 'vitest'

describe('v-text', () => {
  test('should convert v-text to setText', () => {
    const { code, helpers } = compile(`<div v-text={str.value}></div>`)

    expect(helpers).contains('setText')

    expect(code).matchSnapshot()
  })

  test('should raise error and ignore children when v-text is present', () => {
    const onError = vi.fn()
    const { code } = compile(`<div v-text={test}>hello</div>`, {
      onError,
    })
    expect(onError.mock.calls).toMatchObject([[{ code: ErrorCodes.VTextWithChildren }]])

    expect(code).matchSnapshot()
  })

  test('should raise error if has no expression', () => {
    const onError = vi.fn()
    const { code } = compile(`<div v-text></div>`, { onError })
    expect(code).matchSnapshot()
    expect(onError.mock.calls).toMatchObject([[{ code: ErrorCodes.VTextNoExpression }]])
  })
})
