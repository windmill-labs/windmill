import * as csstree from 'css-tree'

export function sanitizeCss(css: string, authorizedClassNames: string[]): string {
	const ast = csstree.parse(css)

	csstree.walk(ast, (node: any, item, list) => {
		if (node.type === 'Rule') {
			let shouldRemoveRule = true

			csstree.walk(node, (innerNode: any) => {
				if (innerNode.type === 'ClassSelector' && authorizedClassNames.includes(innerNode.name)) {
					shouldRemoveRule = false
				}
			})

			if (shouldRemoveRule) {
				list.remove(item)
			}
		}
	})

	return csstree.generate(ast)
}
