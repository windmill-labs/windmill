import * as csstree from 'css-tree'

export function sanitizeCss(css: string, authorizedClassNames: string[]) {
	const ast = csstree.parse(css)
	const removedClassNames: string[] = []

	csstree.walk(ast, (node: any, item, list) => {
		if (node.type === 'Rule') {
			let shouldRemoveRule = true

			csstree.walk(node, (innerNode: any) => {
				if (innerNode.type === 'ClassSelector' && authorizedClassNames.includes(innerNode.name)) {
					shouldRemoveRule = false
				}
				if (shouldRemoveRule && innerNode.name) {
					removedClassNames.push(innerNode.name)
				}
			})

			if (shouldRemoveRule) {
				list.remove(item)
			}
		}
	})

	return {
		css: csstree.generate(ast),
		removedClassNames
	}
}
