import { extend, isString } from '@vue/shared'
import { generate, type VaporCodegenResult } from './generate'
import { IRNodeTypes, type RootNode } from './ir'
import {
  transform,
  type DirectiveTransform,
  type NodeTransform,
  type TransformOptions,
} from './transform'
import { transformChildren } from './transforms/transformChildren'
import { transformElement } from './transforms/transformElement'
import { transformTemplateRef } from './transforms/transformTemplateRef'
import { transformText } from './transforms/transformText'
import { transformVBind } from './transforms/vBind'
import { transformVFor } from './transforms/vFor'
import { transformVHtml } from './transforms/vHtml'
import { transformVIf } from './transforms/vIf'
import { transformVModel } from './transforms/vModel'
import { transformVOn } from './transforms/vOn'
import { transformVOnce } from './transforms/vOnce'
import { transformVShow } from './transforms/vShow'
import { transformVSlot } from './transforms/vSlot'
import { transformVSlots } from './transforms/vSlots'
import { transformVText } from './transforms/vText'
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
  const ast: RootNode = {
    type: IRNodeTypes.ROOT,
    children,
    source: resolvedOptions.source || '',
  }
  const [nodeTransforms, directiveTransforms] = getBaseTransformPreset()

  const ir = transform(
    ast,
    extend({}, resolvedOptions, {
      nodeTransforms: [
        ...nodeTransforms,
        ...(resolvedOptions.nodeTransforms || []), // user transforms
      ],
      directiveTransforms: extend(
        {},
        directiveTransforms,
        resolvedOptions.directiveTransforms || {}, // user transforms
      ),
    }),
  )

  return generate(ir, resolvedOptions)
}

export type CompilerOptions = TransformOptions
export type TransformPreset = [
  NodeTransform[],
  Record<string, DirectiveTransform>,
]

export function getBaseTransformPreset(): TransformPreset {
  return [
    [
      transformVOnce,
      transformVIf,
      transformVFor,
      transformTemplateRef,
      transformElement,
      transformText,
      transformVSlot,
      transformChildren,
    ],
    {
      bind: transformVBind,
      on: transformVOn,
      model: transformVModel,
      show: transformVShow,
      html: transformVHtml,
      text: transformVText,
      slots: transformVSlots,
    },
  ]
}
