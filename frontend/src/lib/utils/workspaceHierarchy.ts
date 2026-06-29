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
	const workspaceMap = new Map(workspaces.map((w) => [w.id, w]))
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
	const rootWorkspaces = workspaces.filter((w) => {
		if (!w.parent_workspace_id) {
			return true // Definitely a root
		}
		// Check if parent exists in the current workspace list
		return !workspaceMap.has(w.parent_workspace_id)
	})

	const result: WorkspaceHierarchyItem[] = []

	// Recursively build the hierarchy
	function addWorkspaceAndChildren(
		workspace: UserWorkspace,
		depth: number,
		isForked: boolean,
		parentName?: string
	) {
		// Add the current workspace
		result.push({
			workspace,
			depth,
			isForked,
			parentName,
			hasChildren: hasChildrenSet.has(workspace.id)
		})

		// Add its children: the canonical dev workspace first, then throwaway forks by name.
		const children = childrenMap.get(workspace.id) || []
		children
			.sort((a, b) => {
				if (!!a.is_dev_workspace !== !!b.is_dev_workspace) return a.is_dev_workspace ? -1 : 1
				return a.name.localeCompare(b.name)
			})
			.forEach((child) => {
				addWorkspaceAndChildren(child, depth + 1, true, workspace.name)
			})
	}

	// Process root workspaces (sorted by name for consistency)
	rootWorkspaces
		.sort((a, b) => a.name.localeCompare(b.name))
		.forEach((workspace) => {
			const isRootForked = workspace.parent_workspace_id != null
			const parentName =
				isRootForked && workspace.parent_workspace_id
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
 * Whether a workspace (by id) is a fork or dev workspace. Forks and dev workspaces both set
 * `parent_workspace_id` (a dev workspace has no `wm-fork-` id prefix), but a `wm-fork-` workspace can
 * outlive its parent (the parent FK is `ON DELETE SET NULL`), so treat the prefix as fork-ness too —
 * otherwise an orphaned fork would lose its fork-only affordances (e.g. owner self-delete).
 */
export function workspaceIsFork(
	workspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): boolean {
	if (!workspaceId) return false
	if (workspaceId.startsWith('wm-fork-')) return true
	return allWorkspaces.find((w) => w.id === workspaceId)?.parent_workspace_id != null
}

/**
 * The canonical dev workspace of a prod workspace, if any (at most one per prod). Used to redirect
 * edits from a locked prod workspace into its dev workspace. Disabled dev workspaces are excluded:
 * redirecting edits to one the user can't select would be a dead end.
 */
export function findCanonicalDevWorkspace(
	prodWorkspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): UserWorkspace | undefined {
	if (!prodWorkspaceId) return undefined
	return allWorkspaces.find(
		(w) => w.parent_workspace_id === prodWorkspaceId && w.is_dev_workspace && !w.disabled
	)
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
