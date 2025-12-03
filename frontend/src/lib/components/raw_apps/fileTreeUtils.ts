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
		// Check if this path represents a folder (ends with /)
		const pathEndsWithSlash = filePath.endsWith('/')
		const parts = filePath.split('/').filter(Boolean)
		let currentPath = ''
		let parentChildren = root

		for (let i = 0; i < parts.length; i++) {
			const part = parts[i]
			currentPath = currentPath ? `${currentPath}/${part}` : part
			// It's a folder if it's not the last part, or if the original path ended with /
			const isFolder = i < parts.length - 1 || (i === parts.length - 1 && pathEndsWithSlash)
			const isLastPart = i === parts.length - 1

			// Check if this node already exists
			if (!nodeMap.has(currentPath)) {
				// Build the node path with trailing / for folders
				let nodePath = '/' + currentPath
				if (isFolder && isLastPart && pathEndsWithSlash) {
					nodePath = nodePath + '/'
				}

				const node: TreeNode = {
					name: part,
					path: nodePath,
					isFolder,
					children: isFolder ? [] : undefined
				}

				nodeMap.set(currentPath, node)
				parentChildren.push(node)
			} else if (isFolder) {
				// If the node exists but wasn't marked as a folder, update it
				const existingNode = nodeMap.get(currentPath)!
				if (!existingNode.isFolder) {
					existingNode.isFolder = true
					existingNode.children = []
					// Update path to include trailing /
					if (isLastPart && pathEndsWithSlash && !existingNode.path.endsWith('/')) {
						existingNode.path = existingNode.path + '/'
					}
				}
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
