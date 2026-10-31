import { logFeatureUsage } from '$lib/utils/featureUsage'

/**
 * Working in: the owner roots, `f/<folder>` or `u/<user>`, a session's AI changes items in, or
 * the whole workspace. A guardrail against the AI's mistakes, not a permission: reads stay
 * open, runs follow the autonomy setting, and a change outside the scope stops on a card the
 * user answers, YOLO included.
 */

const ROOT = /^([fu])\/([^/]+)(?:\/|$)/

/** The owner root a workspace path lives under, or undefined for a path outside `f/` and
 * `u/` (a hub path, a group), which no scope covers. */
export function ownerRoot(path: string): string | undefined {
	const match = ROOT.exec(path.replace(/^\$(?:res|var):/, '').replace(/^\/+/, ''))
	return match ? `${match[1]}/${match[2]}` : undefined
}

/** The scope a session opened from an item starts in: its folder, for an item under `f/`. */
export function presetScopeFor(path: string | undefined): string[] | undefined {
	const root = path ? ownerRoot(path) : undefined
	return root?.startsWith('f/') ? [root] : undefined
}

/** How a scope reads in copy: `f/a`, `f/a and f/b`, `f/a, f/b and f/c`. */
export function scopeLabel(roots: readonly string[]): string {
	return roots.length < 2
		? (roots[0] ?? '')
		: `${roots.slice(0, -1).join(', ')} and ${roots[roots.length - 1]}`
}

export type WorkingScopeIdentity = {
	username: string
	is_admin?: boolean
	/** The folders the user can write. */
	folders?: string[]
}

/** Argument names that carry the path a call writes. */
const PATH_ARGUMENTS = ['path', 'app_path', 'resource_path']

/** Bags carried through to somewhere else — a runnable's arguments, a resource's value. A
 * `path` inside one is the payload's, not a workspace path. */
const OPAQUE_ARGUMENTS = ['args', 'arguments', 'staticInputs', 'value']

/** The one nested bag read: `write_trigger` takes its whole request inside `config`. Elsewhere
 * a nested path is a reference the call does not write (the script an app runnable calls). */
const REQUEST_BAG = 'config'

/** The workspace paths a call writes. */
export function writtenPaths(args: unknown): string[] {
	const paths: string[] = []
	const visit = (candidate: unknown, depth: number, parentKey?: string) => {
		if (!candidate || typeof candidate !== 'object') return
		for (const [key, child] of Object.entries(candidate)) {
			if (
				typeof child === 'string' &&
				PATH_ARGUMENTS.includes(key) &&
				(depth === 0 || parentKey === REQUEST_BAG)
			) {
				paths.push(child)
			} else if (typeof child === 'object' && !OPAQUE_ARGUMENTS.includes(key)) {
				visit(child, depth + 1, key)
			}
		}
	}
	visit(args, 0)
	return [...new Set(paths)]
}

/** `run_script`, `test_run_step`, `test_pipeline_node`…: every tool that runs code is named so,
 * and runs are the autonomy setting's to govern, in scope or not. */
const RUN_TOOL = /^(run|test)_/

export function isRunTool(toolName: string): boolean {
	return RUN_TOOL.test(toolName)
}

/** The verb the block row uses for a call. */
export function scopeVerb(toolName: string): string {
	if (toolName.startsWith('delete_')) return 'delete'
	if (toolName.startsWith('deploy_')) return 'deploy'
	if (toolName.startsWith('create_') || toolName.startsWith('init_')) return 'create'
	return 'edit'
}

export class WorkingScope {
	constructor(
		/** The roots worked in, undefined for the whole workspace. Empty: none, so every change
		 * asks. */
		readonly roots: readonly string[] | undefined,
		readonly identity: WorkingScopeIdentity | undefined
	) {}

	/** The scope in copy, empty for the whole workspace. */
	get label(): string {
		if (!this.roots) return ''
		return this.roots.length ? scopeLabel(this.roots) : 'the selected folders'
	}

	includes(path: string): boolean {
		const root = ownerRoot(path)
		return !root || !this.roots || this.roots.includes(root)
	}

	/** Whether a skill saved at `path` is loaded: those in other folders are suspended while
	 * scoped, the user's own space stays on. */
	loadsSkill(path: string): boolean {
		return this.includes(path) || ownerRoot(path) === `u/${this.identity?.username}`
	}

	/** The first path a write with these arguments lands on outside the scope. */
	outsidePath(args: unknown): string | undefined {
		if (!this.roots) return undefined
		return writtenPaths(args).find((p) => !this.includes(p))
	}

	/** The system prompt section, empty for the whole workspace. `instructions` is the scoped
	 * folder's own instructions, already formatted. */
	promptSection(instructions: string): string {
		if (!this.roots) return ''
		if (!this.roots.length) {
			return `\n\nWORKING IN NO FOLDER:
The user selected no folder for this session. A create, edit, move, delete or deploy does not run: Windmill asks the user, who can allow it once or allow writes in that folder. When the task needs one, make the call directly rather than asking first. Read anything you need, and runs are not limited.`
		}
		const dirs = this.roots.map((r) => `\`${r}/\``).join(', ')
		const one = this.roots.length === 1
		return `\n\nWORKING IN ${dirs}:
The user scoped this session to ${dirs}.
- Create new items under ${one ? dirs : 'one of them (the one the task belongs to)'}, and change only items there. A create, edit, move, delete or deploy elsewhere does not run: Windmill asks the user, who can allow it once or allow writes in that folder from then on. When the task needs one, make the call directly rather than asking first, and never move a change to another folder to avoid the question.
- Read anything you need from any folder: a flow here can call scripts and use resources from other folders. Runs are not limited by the scope.
- list_workspace_items lists under ${dirs} unless you pass path_prefix; pass path_prefix "" to list the whole workspace when ${one ? 'the folder lacks' : 'they lack'} what you need.
- If the user cannot write a folder of the scope, creating there fails: say so and suggest another scope.${instructions ? `\n\nThe instructions of ${one ? 'the folder' : 'these folders'}, which apply to all your work in ${one ? 'it' : 'them'}:\n${instructions}` : ''}`
	}
}

/** Whether the user can write under `root`, for the picker's read-only tag. Unknown folder
 * rights count as writable, so a failed lookup marks nothing read only. */
export function canWriteRoot(root: string, identity: WorkingScopeIdentity | undefined): boolean {
	if (!identity?.folders || identity.is_admin) return true
	const [kind, name] = root.split('/')
	if (kind === 'u') return name === identity.username
	return identity.folders.includes(name)
}

/** What the user answered on the block row. Stopping the turn leaves it unanswered. */
export type ScopeDecision = 'once' | 'add'

/** What the model is told after the user let a call through, so it neither says the call
 * needed no confirmation nor assumes the next one is allowed too. */
export function scopeAllowedNote(
	path: string,
	root: string,
	scope: string,
	decision: ScopeDecision
): string {
	return decision === 'once'
		? `The user allowed this one change to ${path}, outside ${scope}: the next change outside ${scope} asks again.`
		: `The user allowed writes in ${root} for the rest of this session: changes there no longer ask.`
}

/** The tool result of a call the turn was stopped on. */
export function scopeStoppedResult(path: string, scope: string): string {
	return `Not done: ${path} is outside ${scope}, where this session works, and the user stopped instead of allowing it. Nothing was changed.`
}

export type ScopeOption = {
	root: string
	/** The user cannot write here, so creating fails. */
	readOnly: boolean
	instructions: string[]
	skills: string[]
	mcpServers: string[]
}

export type ScopeResources = {
	instructions: readonly { path: string }[]
	skills: readonly { path: string }[]
	mcpServers: readonly { path: string }[]
}

/** The roots the picker offers: the user's own space, then every folder they can read. */
export function scopeOptions(
	folders: readonly string[],
	identity: WorkingScopeIdentity | undefined,
	resources: ScopeResources
): ScopeOption[] {
	const roots = [
		...(identity?.username ? [`u/${identity.username}`] : []),
		...[...folders].sort().map((f) => `f/${f}`)
	]
	return roots.map((root) => scopeOption(root, identity, resources))
}

export function scopeOption(
	root: string,
	identity: WorkingScopeIdentity | undefined,
	resources: ScopeResources
): ScopeOption {
	const under = (list: readonly { path: string }[]) =>
		list.filter((r) => ownerRoot(r.path) === root).map((r) => r.path)
	return {
		root,
		readOnly: !canWriteRoot(root, identity),
		instructions: under(resources.instructions),
		skills: under(resources.skills),
		mcpServers: under(resources.mcpServers)
	}
}

function plural(n: number, word: string): string {
	return `${n} ${word}${n === 1 ? '' : 's'}`
}

/** `1 instruction · 2 skills · 1 MCP`, empty when the root carries none. */
export function scopeBadge(option: ScopeOption): string {
	return [
		...(option.instructions.length ? [plural(option.instructions.length, 'instruction')] : []),
		...(option.skills.length ? [plural(option.skills.length, 'skill')] : []),
		...(option.mcpServers.length ? [`${option.mcpServers.length} MCP`] : [])
	].join(' · ')
}

/** How a folder's instructions and skills reach the AI, in the one wording the picker and the
 * tree badge share. */
export function scopeRule(root: string): string {
	return `Working in ${root} puts its instructions in the AI's prompt and keeps its skills on; skills from other folders are off. Outside it, the AI gets a folder's instructions the first time it touches that folder.`
}

export type ScopeKind = 'workspace' | 'none' | 'folder' | 'user_space' | 'several'
export type ScopeSource = 'default' | 'preset' | 'manual'

export function scopeKind(roots: readonly string[] | undefined): ScopeKind {
	if (!roots) return 'workspace'
	if (!roots.length) return 'none'
	if (roots.length > 1) return 'several'
	return roots[0].startsWith('u/') ? 'user_space' : 'folder'
}

/** Closed vocabulary for the `ai_session`/`working_folders` counter: a scope a turn was sent
 * in, a block's outcome, and how many folders a whole-workspace session wrote to (`1`, then
 * `2+` once it reaches a second). */
export type WorkingScopeEvent =
	| `scope:${ScopeKind}:${ScopeSource}`
	| `block:${'once' | 'add' | 'none'}`
	| `written:${'1' | '2+'}`

export function logWorkingScope(key: WorkingScopeEvent, workspace: string | undefined): void {
	logFeatureUsage('ai_session', 'working_folders', { key, workspace })
}
