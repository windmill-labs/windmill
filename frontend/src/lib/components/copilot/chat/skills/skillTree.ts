/** Grouping for the Skills list: skills sorted into the folders that hold them.
 *
 * Only worth showing when the skills are spread over more than one folder — see
 * `skillFolderPaths` — since a tree over a single folder is a list with a header
 * on top of it. */

/** A folder in the skills tree. `path` is the whole prefix the node covers, so it
 * is unique across the tree and can key collapsed state. */
export type SkillTreeNode<T> = {
	path: string
	/** Header text: the whole scope (`u/admin`, `f/skills`) at the root, one
	 * segment (`deploy`) deeper, where the ancestors are already on screen. */
	label: string
	/** A `u/<user>`, `f/<folder>` or `g/<group>` root rather than a subfolder. */
	scope: boolean
	children: SkillTreeNode<T>[]
	skills: T[]
}

type Skill = { path: string; name: string }

/** The folders holding these skills — one entry per distinct parent path. */
export function skillFolderPaths(skills: readonly Skill[]): Set<string> {
	return new Set(skills.map((s) => s.path.split('/').slice(0, -1).join('/')))
}

/** Every skill under `node`, its subfolders included. */
export function countSkills<T>(node: SkillTreeNode<T>): number {
	return node.skills.length + node.children.reduce((n, c) => n + countSkills(c), 0)
}

/** The skills a folder switch acts on: everything under it, however deep. */
export function nodeSkills<T>(node: SkillTreeNode<T>): T[] {
	return [...node.skills, ...node.children.flatMap((c) => nodeSkills(c))]
}

/** One line of the rendered list, folders included. */
export type SkillTreeEntry<T> = { key: string; parentKey?: string } & (
	| { kind: 'folder'; node: SkillTreeNode<T> }
	| { kind: 'skill'; skill: T }
)

/** A folder and a skill can hold the same path — `f/skills/deploy` names both when
 * `f/skills/deploy/rollback` exists beside it — so the kind is part of the key. */
export function folderKey(path: string): string {
	return `folder:${path}`
}
export function skillKey(path: string): string {
	return `skill:${path}`
}

/** The list as it stands on screen, top to bottom, with collapsed folders holding
 * their contents back. This is what the keyboard walks; the markup renders the same
 * forest recursively under the same `isCollapsed`, so the two agree line for line. */
export function visibleEntries<T extends Skill>(
	forest: readonly SkillTreeNode<T>[],
	isCollapsed: (path: string) => boolean
): SkillTreeEntry<T>[] {
	const out: SkillTreeEntry<T>[] = []
	const walk = (node: SkillTreeNode<T>, parentKey?: string) => {
		const key = folderKey(node.path)
		out.push({ kind: 'folder', key, parentKey, node })
		if (isCollapsed(node.path)) return
		for (const child of node.children) walk(child, key)
		for (const skill of node.skills) {
			out.push({ kind: 'skill', key: skillKey(skill.path), parentKey: key, skill })
		}
	}
	for (const root of forest) walk(root)
	return out
}

/** Sort a skill's path into the forest, creating the folders it names.
 *
 * Paths are `[ufg]/<owner>/<name>` and may nest further (`f/skills/deploy/rollback`
 * passes the resource path CHECK), so the first two segments are one node and each
 * segment after that is a node of its own. */
export function buildSkillTree<T extends Skill>(skills: readonly T[]): SkillTreeNode<T>[] {
	const roots = new Map<string, SkillTreeNode<T>>()
	for (const skill of skills) {
		const parts = skill.path.split('/')
		// Two segments cannot happen through the API, but a hand-written path in a
		// test or a future path shape should land somewhere rather than vanish.
		const rootPath = parts.slice(0, 2).join('/')
		let node: SkillTreeNode<T> = roots.get(rootPath) ?? {
			path: rootPath,
			label: rootPath,
			scope: true,
			children: [],
			skills: []
		}
		roots.set(rootPath, node)
		for (const segment of parts.slice(2, -1)) {
			const path = `${node.path}/${segment}`
			let child = node.children.find((c) => c.path === path)
			if (child === undefined) {
				child = { path, label: segment, scope: false, children: [], skills: [] }
				node.children.push(child)
			}
			node = child
		}
		node.skills.push(skill)
	}

	const sort = (node: SkillTreeNode<T>) => {
		node.children.sort((a, b) => a.label.localeCompare(b.label))
		node.skills.sort((a, b) => a.name.localeCompare(b.name))
		node.children.forEach(sort)
	}
	const forest = [...roots.values()]
	forest.forEach(sort)
	// Your own folder first, then the shared ones alphabetically: `u/<me>` is where
	// this modal's New skill and folder import write.
	return forest.sort((a, b) => {
		const scopeRank = (p: string) => (p.startsWith('u/') ? 0 : 1)
		return scopeRank(a.path) - scopeRank(b.path) || a.path.localeCompare(b.path)
	})
}
