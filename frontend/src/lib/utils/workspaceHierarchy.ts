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
 * Walk up `parent_workspace_id` to the top of a workspace's family. Stops at the first ancestor not
 * present in `allWorkspaces` (e.g. a parent the user can't see) and returns it, so the result is
 * always the highest reachable ancestor. Returns undefined when the id itself isn't in the list.
 *
 * A dev workspace nested under another dev workspace (a dev of a dev, supported but not the
 * recommended shape) ends the walk: it is the prod of everything below it, and presenting the far
 * root — which such a family may promote to only through an intermediate, and which its members
 * often can't even reach — as the family head makes every root-scoped affordance (fork base, deploy
 * target, scope chip) point past the workspace actually being worked in.
 */
export function findWorkspaceRoot(
	workspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): UserWorkspace | undefined {
	if (!workspaceId) return undefined
	let current = allWorkspaces.find((w) => w.id === workspaceId)
	while (current?.parent_workspace_id) {
		const parent = allWorkspaces.find((w) => w.id === current!.parent_workspace_id)
		if (!parent) break
		const crossedNestedDev = !!current.is_dev_workspace && !!parent.is_dev_workspace
		current = parent
		if (crossedNestedDev) break
	}
	return current
}

/**
 * Every reachable ancestor of a workspace, nearest first. Unlike `findWorkspaceRoot` this never stops
 * at a dev-of-dev boundary — it answers "is X above me in the real tree?", e.g. to keep an ancestor
 * out of a dev-workspace attach list, where reparenting it below would close a cycle.
 */
export function findWorkspaceAncestors(
	workspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): UserWorkspace[] {
	const ancestors: UserWorkspace[] = []
	let current = workspaceId ? allWorkspaces.find((w) => w.id === workspaceId) : undefined
	const seen = new Set<string>(current ? [current.id] : [])
	while (current?.parent_workspace_id) {
		const parent = allWorkspaces.find((w) => w.id === current!.parent_workspace_id)
		if (!parent || seen.has(parent.id)) break
		seen.add(parent.id)
		ancestors.push(parent)
		current = parent
	}
	return ancestors
}

/**
 * The dev workspaces at or above `workspaceId`. A dev workspace placed under it joins their
 * promotion chain, so their environment labels are the ones it cannot reuse — dev workspaces in a
 * chain inherit the same git-sync repositories, and an equal label means one shared deploy branch.
 * `disabled` is not filtered out, unlike elsewhere: it means the caller has no seat in that
 * workspace, which does not free the branch it deploys to. Ancestors the caller cannot see end the
 * walk, so this can under-report; the backend rejects on the full tree either way.
 */
export function devWorkspacesInChainAbove(
	workspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): UserWorkspace[] {
	const self = workspaceId ? allWorkspaces.find((w) => w.id === workspaceId) : undefined
	return [...(self ? [self] : []), ...findWorkspaceAncestors(workspaceId, allWorkspaces)].filter(
		(w) => w.is_dev_workspace
	)
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
 * Whether `userEmail` is the creator of a fork workspace. The fork creator gets workspace-settings
 * access (the fork members screen) even when they are not an admin of it: forking as an ordinary
 * developer copies their parent `usr` row, leaving them otherwise unable to bring collaborators in.
 * Mirrors the backend `authorize_fork_owner_add_user` grant.
 */
export function isForkOwner(
	workspace: UserWorkspace | undefined,
	userEmail: string | null | undefined
): boolean {
	return (
		Boolean(workspace?.parent_workspace_id) && !!userEmail && workspace?.created_by === userEmail
	)
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
 * The workspace a new fork should branch from by default. Inside a dev workspace's subtree the base is
 * that dev workspace, not the family root: the dev workspace carries the changes being worked on, and
 * a prod locked against forking (`lock_prod_forking`) rejects a root-based fork outright. Anywhere
 * else it is the family root, so an ad-hoc fork branches from prod rather than from whichever
 * throwaway fork happens to be open. A dev workspace the user is disabled in is skipped — forking from
 * it would fail — and the walk continues upwards.
 */
export function findDefaultForkBase(
	currentWorkspaceId: string | undefined,
	allWorkspaces: UserWorkspace[]
): UserWorkspace | undefined {
	let current = allWorkspaces.find((w) => w.id === currentWorkspaceId)
	while (current) {
		if (current.is_dev_workspace && !current.disabled) return current
		if (!current.parent_workspace_id) break
		const parent: UserWorkspace | undefined = allWorkspaces.find(
			(w) => w.id === current!.parent_workspace_id
		)
		if (!parent) break
		current = parent
	}
	return findWorkspaceRoot(currentWorkspaceId, allWorkspaces)
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
