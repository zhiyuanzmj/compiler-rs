import {
  getDelimitersArrayNewline,
  getDelimitersObject,
  getDelimitersObjectNewline,
} from '@vue-jsx-vapor/compiler-rs'
import { camelize, isArray } from '@vue/shared'
import {
  IRDynamicPropsKind,
  IRSlotType,
  type BlockIRNode,
  type CreateComponentIRNode,
  type IRProp,
  type IRProps,
  type IRPropsStatic,
  type IRSlotDynamic,
  type IRSlotDynamicBasic,
  type IRSlotDynamicConditional,
  type IRSlotDynamicLoop,
  type IRSlots,
  type IRSlotsStatic,
} from '../ir'
import {
  createSimpleExpression,
  genCall,
  genMulti,
  INDENT_END,
  INDENT_START,
  NEWLINE,
  toValidAssetId,
  walkIdentifiers,
  type CodeFragment,
} from '../utils'
import type { CodegenContext } from '../generate'
import { genBlock } from './block'
import { genDirectiveModifiers, genDirectivesForElement } from './directive'
import { genEventHandler } from './event'
import { genExpression } from './expression'
import { genPropKey, genPropValue } from './prop'
import { genModelHandler } from './vModel'

export function genCreateComponent(
  operation: CreateComponentIRNode,
  context: CodegenContext,
): CodeFragment[] {
  const { helper } = context

  const tag = genTag()
  const { root, props, slots, once } = operation
  const rawProps = genRawProps(props, context)
  const rawSlots = genRawSlots(slots, context)

  return [
    NEWLINE,
    `const n${operation.id} = `,
    ...genCall(
      operation.dynamic && !operation.dynamic.isStatic
        ? helper('createDynamicComponent')
        : operation.asset
          ? helper('createComponentWithFallback')
          : helper('createComponent'),
      [tag, rawProps, rawSlots, root ? 'true' : null, once ? 'true' : null],
    ),
    ...genDirectivesForElement(operation.id, context),
  ]

  function genTag() {
    if (operation.dynamic) {
      if (operation.dynamic.isStatic) {
        return genCall(helper('resolveDynamicComponent'), [
          genExpression(operation.dynamic, context),
        ])
      } else {
        return ['() => (', ...genExpression(operation.dynamic, context), ')']
      }
    } else if (operation.asset) {
      return toValidAssetId(operation.tag, 'component')
    } else {
      return genExpression(createSimpleExpression(operation.tag), context)
    }
  }
}

export function genRawProps(
  props: IRProps[],
  context: CodegenContext,
): CodeFragment[] | undefined {
  const staticProps = props[0]
  if (isArray(staticProps)) {
    if (!staticProps.length && props.length === 1) {
      return
    }
    return genStaticProps(
      staticProps,
      context,
      genDynamicProps(props.slice(1), context),
    )
  } else if (props.length) {
    // all dynamic
    return genStaticProps([], context, genDynamicProps(props, context))
  }
}

function genStaticProps(
  props: IRPropsStatic,
  context: CodegenContext,
  dynamicProps?: CodeFragment[],
): CodeFragment[] {
  const args = props.map((prop) => genProp(prop, context, true))
  if (dynamicProps) {
    args.push([`$: `, ...dynamicProps])
  }
  return genMulti(
    args.length > 1 ? getDelimitersObjectNewline() : getDelimitersObject(),
    args,
  )
}

function genDynamicProps(
  props: IRProps[],
  context: CodegenContext,
): CodeFragment[] | undefined {
  const { helper } = context
  const frags: CodeFragment[][] = []
  for (const p of props) {
    let expr: CodeFragment[]
    if (isArray(p)) {
      if (p.length) {
        frags.push(genStaticProps(p, context))
      }
      continue
    } else if (p.kind === IRDynamicPropsKind.ATTRIBUTE)
      expr = genMulti(getDelimitersObject(), [genProp(p, context)])
    else {
      expr = genExpression(p.value, context)
      if (p.handler) expr = genCall(helper('toHandlers'), [expr])
    }
    frags.push(['() => (', ...expr, ')'])
  }
  if (frags.length) {
    return genMulti(getDelimitersArrayNewline(), frags)
  }
}

function genProp(prop: IRProp, context: CodegenContext, isStatic?: boolean) {
  const values = genPropValue(prop.values, context)
  return [
    ...genPropKey(prop, context),
    ': ',
    ...(prop.handler
      ? genEventHandler(
          context,
          prop.values[0],
          prop.handlerModifiers,
          true /* wrap handlers passed to components */,
        )
      : isStatic
        ? ['() => (', ...values, ')']
        : values),
    ...(prop.model
      ? [...genModelEvent(prop, context), ...genModelModifiers(prop, context)]
      : []),
  ]
}

function genModelEvent(prop: IRProp, context: CodegenContext): CodeFragment[] {
  const name = prop.key.isStatic
    ? [JSON.stringify(`onUpdate:${camelize(prop.key.content)}`)]
    : ['["onUpdate:" + ', ...genExpression(prop.key, context), ']']
  const handler = genModelHandler(prop.values[0], context)

  return [',', NEWLINE, ...name, ': () => ', ...handler]
}

function genModelModifiers(
  prop: IRProp,
  context: CodegenContext,
): CodeFragment[] {
  const { key, modelModifiers } = prop
  if (!modelModifiers || !modelModifiers.length) return []

  const modifiersKey = key.isStatic
    ? [`${key.content}Modifiers`]
    : ['[', ...genExpression(key, context), ' + "Modifiers"]']

  const modifiersVal = genDirectiveModifiers(modelModifiers)
  return [',', NEWLINE, ...modifiersKey, `: () => ({ ${modifiersVal} })`]
}

function genRawSlots(slots: IRSlots[], context: CodegenContext) {
  if (!slots.length) return
  const staticSlots = slots[0]
  if (staticSlots.slotType === IRSlotType.STATIC) {
    // single static slot
    return genStaticSlots(
      staticSlots,
      context,
      slots.length > 1 ? slots.slice(1) : undefined,
    )
  } else {
    return genStaticSlots(
      { slotType: IRSlotType.STATIC, slots: {} },
      context,
      slots,
    )
  }
}

function genStaticSlots(
  { slots }: IRSlotsStatic,
  context: CodegenContext,
  dynamicSlots?: IRSlots[],
) {
  const args = Object.keys(slots).map((name) => [
    `${JSON.stringify(name)}: `,
    ...genSlotBlockWithProps(slots[name], context),
  ])
  if (dynamicSlots) {
    args.push([`$: `, ...genDynamicSlots(dynamicSlots, context)])
  }
  return genMulti(getDelimitersObjectNewline(), args)
}

function genDynamicSlots(
  slots: IRSlots[],
  context: CodegenContext,
): CodeFragment[] {
  return genMulti(
    getDelimitersArrayNewline(),
    slots.map((slot) =>
      slot.slotType === IRSlotType.STATIC
        ? genStaticSlots(slot, context)
        : slot.slotType === IRSlotType.EXPRESSION
          ? slot.slots.content
          : genDynamicSlot(slot, context, true),
    ),
  )
}

function genDynamicSlot(
  slot: IRSlotDynamic,
  context: CodegenContext,
  withFunction = false,
): CodeFragment[] {
  let frag: CodeFragment[]
  switch (slot.slotType) {
    case IRSlotType.DYNAMIC:
      frag = genBasicDynamicSlot(slot, context)
      break
    case IRSlotType.LOOP:
      frag = genLoopSlot(slot, context)
      break
    case IRSlotType.CONDITIONAL:
      frag = genConditionalSlot(slot, context)
      break
  }
  return withFunction ? ['() => (', ...frag, ')'] : frag
}

function genBasicDynamicSlot(
  slot: IRSlotDynamicBasic,
  context: CodegenContext,
): CodeFragment[] {
  const { name, fn } = slot
  return genMulti(getDelimitersObjectNewline(), [
    ['name: ', ...genExpression(name, context)],
    ['fn: ', ...genSlotBlockWithProps(fn, context)],
  ])
}

function genLoopSlot(
  slot: IRSlotDynamicLoop,
  context: CodegenContext,
): CodeFragment[] {
  const { name, fn, loop } = slot
  const { value, key, index, source } = loop
  const rawValue = value && value.content
  const rawKey = key && key.content
  const rawIndex = index && index.content

  const idMap: Record<string, string> = {}
  if (rawValue) idMap[rawValue] = rawValue
  if (rawKey) idMap[rawKey] = rawKey
  if (rawIndex) idMap[rawIndex] = rawIndex
  const slotExpr = genMulti(getDelimitersObjectNewline(), [
    ['name: ', ...context.withId(() => genExpression(name, context), idMap)],
    [
      'fn: ',
      ...context.withId(() => genSlotBlockWithProps(fn, context), idMap),
    ],
  ])
  return [
    ...genCall(context.helper('createForSlots'), [
      genExpression(source!, context),
      [
        ...genMulti(
          ['(', ')', ', ', undefined],
          [
            rawValue ? rawValue : rawKey || rawIndex ? '_' : null,
            rawKey ? rawKey : rawIndex ? '__' : null,
            rawIndex,
          ],
        ),
        ' => (',
        ...slotExpr,
        ')',
      ],
    ]),
  ]
}

function genConditionalSlot(
  slot: IRSlotDynamicConditional,
  context: CodegenContext,
): CodeFragment[] {
  const { condition, positive, negative } = slot
  return [
    ...genExpression(condition, context),
    INDENT_START,
    NEWLINE,
    '? ',
    ...genDynamicSlot(positive, context),
    NEWLINE,
    ': ',
    ...(negative ? [...genDynamicSlot(negative, context)] : ['void 0']),
    INDENT_END,
  ]
}

function genSlotBlockWithProps(oper: BlockIRNode, context: CodegenContext) {
  let isDestructureAssignment = false
  let rawProps: string | undefined
  let propsName: string | undefined
  let exitScope: (() => void) | undefined
  let depth: number | undefined
  const props = oper.props!
  const idsOfProps = new Set<string>()

  if (props) {
    rawProps = props.content
    if ((isDestructureAssignment = !!props.ast)) {
      ;[depth, exitScope] = context.enterScope()
      propsName = `_slotProps${depth}`
      walkIdentifiers(
        props.ast,
        (id, _, __, isReference, isLocal) => {
          if (isReference && !isLocal) idsOfProps.add(id.name)
        },
        true,
      )
    } else {
      idsOfProps.add((propsName = rawProps))
    }
  }

  const idMap: Record<string, string | null> = {}

  idsOfProps.forEach(
    (id) =>
      (idMap[id] = isDestructureAssignment
        ? `${propsName}[${JSON.stringify(id)}]`
        : null),
  )
  const blockFn = context.withId(
    () => genBlock(oper, context, [propsName]),
    idMap,
  )
  exitScope && exitScope()

  return blockFn
}
