import { extend, isString } from '@vue/shared'
import { generate, type VaporCodegenResult } from './generate'
import { IRNodeTypes } from './ir'
import { transform, type TransformOptions } from './transform'
import { parseExpression } from './utils'
import type { JSXElement, JSXFragment } from 'oxc-parser'

// code/AST -> IR (transform) -> JS (generate)
export function compile(
  source: JSXElement | JSXFragment | string,
  options: CompilerOptions = {},
): VaporCodegenResult {
  const resolvedOptions = extend({}, options, {
    filename: 'index.jsx',
  })
  if (!resolvedOptions.source && isString(source)) {
    resolvedOptions.source = source
  }
  const root = isString(source)
    ? parseExpression(resolvedOptions.filename, source)
    : source
  const children =
    root.type === 'JSXFragment'
      ? root.children
      : root.type === 'JSXElement'
        ? [root]
        : []
  const ast = {
    type: IRNodeTypes.ROOT,
    children,
  } as unknown as Node

  const ir = transform(ast, extend({}, resolvedOptions))

  return generate(ir, resolvedOptions)
}

export type CompilerOptions = TransformOptions
