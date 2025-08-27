import type { UserWorkspace } from '../stores'

export interface WorkspaceHierarchyItem {
	workspace: UserWorkspace
	depth: number
	isForked: boolean
	parentName?: string
	hasChildren: boolean
}

/**
 * Builds a hierarchical structure from a flat array of workspaces.
 * Supports unlimited nesting levels (fork of fork of fork...).
 * Returns a flattened array with hierarchy metadata for easy rendering.
 */
export function buildWorkspaceHierarchy(workspaces: UserWorkspace[]): WorkspaceHierarchyItem[] {
	if (!workspaces || workspaces.length === 0) {
		return []
	}

	// Create maps for quick lookups
	const workspaceMap = new Map(workspaces.map(w => [w.id, w]))
	const childrenMap = new Map<string, UserWorkspace[]>()
	const hasChildrenSet = new Set<string>()

	// Build children mapping and track which workspaces have children
	for (const workspace of workspaces) {
		if (workspace.parent_workspace_id) {
			if (!childrenMap.has(workspace.parent_workspace_id)) {
				childrenMap.set(workspace.parent_workspace_id, [])
			}
			childrenMap.get(workspace.parent_workspace_id)!.push(workspace)
			hasChildrenSet.add(workspace.parent_workspace_id)
		}
	}

	// Find root workspaces (those without a parent or whose parent is not in the current list)
	const rootWorkspaces = workspaces.filter(w => {
		if (!w.parent_workspace_id) {
			return true // Definitely a root
		}
		// Check if parent exists in the current workspace list
		return !workspaceMap.has(w.parent_workspace_id)
	})

	const result: WorkspaceHierarchyItem[] = []

	// Recursively build the hierarchy
	function addWorkspaceAndChildren(workspace: UserWorkspace, depth: number, isForked: boolean, parentName?: string) {
		// Add the current workspace
		result.push({
			workspace,
			depth,
			isForked,
			parentName,
			hasChildren: hasChildrenSet.has(workspace.id)
		})

		// Add its children (sorted by name for consistency)
		const children = childrenMap.get(workspace.id) || []
		children
			.sort((a, b) => a.name.localeCompare(b.name))
			.forEach(child => {
				addWorkspaceAndChildren(child, depth + 1, true, workspace.name)
			})
	}

	// Process root workspaces (sorted by name for consistency)
	rootWorkspaces
		.sort((a, b) => a.name.localeCompare(b.name))
		.forEach(workspace => {
			const isRootForked = workspace.parent_workspace_id != null
			const parentName = isRootForked && workspace.parent_workspace_id 
				? workspace.parent_workspace_id // Use parent ID as fallback if parent not in list
				: undefined
			
			addWorkspaceAndChildren(workspace, 0, isRootForked, parentName)
		})

	return result
}

/**
 * Helper function to get the indentation padding based on depth.
 * Each level adds 24px of left padding.
 */
export function getWorkspaceIndentation(depth: number): string {
	return `${depth * 24}px`
}

/**
 * Helper function to check if a workspace is a root workspace
 */
export function isRootWorkspace(workspace: UserWorkspace): boolean {
	return workspace.parent_workspace_id == null
}

/**
 * Helper function to find all descendants of a workspace
 */
export function findWorkspaceDescendants(
	workspaceId: string, 
	allWorkspaces: UserWorkspace[]
): UserWorkspace[] {
	const descendants: UserWorkspace[] = []
	const childrenMap = new Map<string, UserWorkspace[]>()

	// Build children mapping
	for (const workspace of allWorkspaces) {
		if (workspace.parent_workspace_id) {
			if (!childrenMap.has(workspace.parent_workspace_id)) {
				childrenMap.set(workspace.parent_workspace_id, [])
			}
			childrenMap.get(workspace.parent_workspace_id)!.push(workspace)
		}
	}

	// Recursively find descendants
	function collectDescendants(id: string) {
		const children = childrenMap.get(id) || []
		for (const child of children) {
			descendants.push(child)
			collectDescendants(child.id)
		}
	}

	collectDescendants(workspaceId)
	return descendants
}