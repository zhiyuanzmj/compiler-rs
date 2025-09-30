import type { IdentifierName, Node } from 'oxc-parser'

export function extractIdentifiers(
  node: Node,
  identifiers: IdentifierName[] = [],
): IdentifierName[] {
  switch (node.type) {
    case 'Identifier':
    case 'JSXIdentifier':
      identifiers.push(node as IdentifierName)
      break

    case 'MemberExpression':
    case 'JSXMemberExpression': {
      let object: any = node
      while (object.type === 'MemberExpression') {
        object = object.object
      }
      identifiers.push(object)
      break
    }

    case 'ObjectPattern':
      for (const prop of node.properties) {
        if (prop.type === 'RestElement') {
          extractIdentifiers(prop.argument, identifiers)
        } else {
          extractIdentifiers(prop.value, identifiers)
        }
      }
      break

    case 'ArrayPattern':
      node.elements.forEach((element) => {
        element && extractIdentifiers(element, identifiers)
      })
      break

    case 'RestElement':
      extractIdentifiers(node.argument, identifiers)
      break

    case 'AssignmentPattern':
      extractIdentifiers(node.left, identifiers)
      break
  }

  return identifiers
}
