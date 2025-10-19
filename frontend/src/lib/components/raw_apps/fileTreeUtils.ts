export interface TreeNode {
	name: string
	path: string
	isFolder: boolean
	children?: TreeNode[]
}

export function buildFileTree(filePaths: string[]): TreeNode[] {
	const root: TreeNode[] = []
	const nodeMap = new Map<string, TreeNode>()

	// Sort paths to ensure parent folders are processed before children
	const sortedPaths = filePaths.slice().sort()

	for (const filePath of sortedPaths) {
		const parts = filePath.split('/').filter(Boolean)
		let currentPath = ''
		let parentChildren = root

		for (let i = 0; i < parts.length; i++) {
			const part = parts[i]
			const previousPath = currentPath
			currentPath = currentPath ? `${currentPath}/${part}` : part
			const isFolder = i < parts.length - 1

			// Check if this node already exists
			if (!nodeMap.has(currentPath)) {
				const node: TreeNode = {
					name: part,
					path: currentPath,
					isFolder,
					children: isFolder ? [] : undefined
				}

				nodeMap.set(currentPath, node)
				parentChildren.push(node)
			}

			// Move to the next level for folders
			if (isFolder) {
				const node = nodeMap.get(currentPath)!
				parentChildren = node.children!
			}
		}
	}

	return root
}
