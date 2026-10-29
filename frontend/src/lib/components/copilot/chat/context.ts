import {
	Code,
	Database,
	TriangleAlert,
	Diff,
	FileCode,
	FileText,
	Code2,
	TextSelect,
	Table2,
	LayoutDashboard,
	MousePointer2,
	Folder,
	Play
} from 'lucide-svelte'
import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
import type { ScriptLang } from '$lib/gen/types.gen'
import { type DBSchema } from '$lib/stores'
import { type Change } from 'diff'
import type { AppDatatableMetadata, BackendRunnable, SelectedContext } from './app/core'
import type { DrillLeaf, DrillNode } from '$lib/components/drillPicker'
import { dirKey } from '$lib/components/workspacePicker'

export const ContextIconMap = {
	code: Code,
	error: TriangleAlert,
	db: Database,
	diff: Diff,
	code_piece: Code,
	app_frontend_file: FileCode,
	app_backend_runnable: Code2,
	app_code_selection: TextSelect,
	app_datatable: Table2,
	app_dom_selector: MousePointer2,
	workspace_script: Code2,
	workspace_flow: BarsStaggered,
	workspace_app: LayoutDashboard,
	workspace_folder: Folder,
	workspace_run: Play,
	// Fallback — the badge renders attached_file with the per-extension file icon.
	attached_file: FileText
	// flow_module type is handled with FlowModuleIcon
}

export interface CodeElement {
	type: 'code'
	content: string
	title: string
	lang: ScriptLang | 'bunnative'
}

interface ErrorElement {
	type: 'error'
	content: string
	title: 'error'
}

interface DBElement {
	type: 'db'
	schema?: DBSchema
	title: string
}

interface DiffElement {
	type: 'diff'
	content: string
	title: string
	diff: Change[]
	lang: ScriptLang | 'bunnative'
}

export interface CodePieceElement {
	type: 'code_piece'
	content: string
	startLine: number
	endLine: number
	title: string
	lang: ScriptLang | 'bunnative'
}

export interface FlowModuleElement {
	type: 'flow_module'
	id: string
	title: string
	// mimics the FlowModule type, with only the fields we need
	value: {
		language?: ScriptLang | 'bunnative'
		path?: string
		content?: string
		type: string
	}
}

export interface FlowModuleCodePieceElement extends Omit<CodePieceElement, 'type'> {
	type: 'flow_module_code_piece'
	id: string
	value: FlowModuleElement['value']
}

/** App frontend file context element */
export interface AppFrontendFileElement {
	type: 'app_frontend_file'
	/** The file path (e.g., /index.tsx, /styles.css) */
	path: string
	/** Title for display (the path) */
	title: string
	/** The file content */
	content: string
}

/** App backend runnable context element */
export interface AppBackendRunnableElement {
	type: 'app_backend_runnable'
	/** The runnable key */
	key: string
	/** Title for display (the key) */
	title: string
	/** The runnable configuration */
	runnable: BackendRunnable
}

/** App code selection context element (from frontend or backend editor) */
export interface AppCodeSelectionElement {
	type: 'app_code_selection'
	/** Source: frontend file path or backend runnable key */
	source: string
	/** Whether this is from frontend or backend */
	sourceType: 'frontend' | 'backend'
	/** Title for display */
	title: string
	/** The selected code content */
	content: string
	/** Line range (1-indexed) */
	startLine: number
	endLine: number
	/** Column range (1-indexed) */
	startColumn: number
	endColumn: number
}

/** App datatable table context element (represents a single table within a datatable) */
export interface AppDatatableElement {
	type: 'app_datatable'
	/** The datatable name (e.g., "main") */
	datatableName: string
	/** The schema name (e.g., "public") */
	schemaName: string
	/** The table name (e.g., "users") */
	tableName: string
	/** Title for display (e.g., "main/public:users" or "main/users") */
	title: string
	/** The table columns: column_name -> compact_type. Loaded only when a table is explicitly selected. */
	columns?: Record<string, string>
}

/**
 * A DOM element the user picked in the raw-app preview inspector, attached to the
 * chat as a lightweight selector reference. The element's HTML is NOT stored — the
 * model reads it live via the `search_dom` / `read_dom` tools using `selector`.
 */
export interface AppDomSelectorElement {
	type: 'app_dom_selector'
	/** Full CSS selector path, passed verbatim to search_dom / read_dom. */
	selector: string
	/** Raw-app path the element was picked from. The selector only makes sense
	 * against this app's preview, so chips are scoped to it (a session can have
	 * several raw-app preview tabs, only one active at a time). */
	appPath: string
	/** Compact, space-free display label, e.g. "button#submit.primary". */
	title: string
	tagName: string
	id?: string
	className?: string
}

export function createAppDomSelectorElement(info: {
	selector: string
	appPath: string
	tagName: string
	id?: string
	className?: string
}): AppDomSelectorElement {
	const tag = info.tagName.toLowerCase()
	const firstClass = info.className?.trim().split(/\s+/)[0]
	const title = `${tag}${info.id ? `#${info.id}` : ''}${firstClass ? `.${firstClass}` : ''}`
	return {
		type: 'app_dom_selector',
		selector: info.selector,
		appPath: info.appPath,
		title,
		tagName: tag,
		id: info.id,
		className: info.className
	}
}

export function createAppSelectedContext(options: SelectedContext = {}): SelectedContext {
	return {
		...options
	}
}

export function createAppFrontendFileContextElement(
	path: string,
	content: string
): AppFrontendFileElement {
	return {
		type: 'app_frontend_file',
		path,
		title: path,
		content
	}
}

export function createAppBackendRunnableContextElement(
	key: string,
	runnable: BackendRunnable
): AppBackendRunnableElement {
	return {
		type: 'app_backend_runnable',
		key,
		title: key,
		runnable
	}
}

export function formatAppDatatableContextTitle(
	datatableName: string,
	schemaName: string,
	tableName: string
): string {
	return schemaName === 'public'
		? `${datatableName}/${tableName}`
		: `${datatableName}/${schemaName}:${tableName}`
}

export function createAppDatatableContextElement(
	datatableName: string,
	schemaName: string,
	tableName: string,
	columns?: Record<string, string>
): AppDatatableElement {
	return {
		type: 'app_datatable',
		datatableName,
		schemaName,
		tableName,
		title: formatAppDatatableContextTitle(datatableName, schemaName, tableName),
		columns
	}
}

export function flattenDatatablesToAppContextElements(
	datatables: AppDatatableMetadata[]
): AppDatatableElement[] {
	return datatables.flatMap((datatable) => {
		if (datatable.error) {
			return []
		}

		return Object.entries(datatable.schemas).flatMap(([schemaName, tables]) =>
			tables.map((tableName) =>
				createAppDatatableContextElement(datatable.datatable_name, schemaName, tableName)
			)
		)
	})
}

/** Workspace script context element — reference to a script in the workspace */
export interface WorkspaceScriptElement {
	type: 'workspace_script'
	path: string
	title: string
	summary?: string
}

/** Workspace flow context element — reference to a flow in the workspace */
export interface WorkspaceFlowElement {
	type: 'workspace_flow'
	path: string
	title: string
	summary?: string
}

/** Workspace app context element — reference to a (code-based) raw app in the
 * workspace. Only raw apps are surfaced: the visual app builder produces a
 * frontend, not a code unit the chat can act on. */
export interface WorkspaceAppElement {
	type: 'workspace_app'
	path: string
	title: string
	summary?: string
}

/** Workspace folder context element — everything under a path prefix: a folder (`f/team`, or a
 * subfolder of an owner) or a user's own items (`u/alice`). Its items are read by the chat on
 * demand like any other workspace reference. */
export interface WorkspaceFolderElement {
	type: 'workspace_folder'
	path: string
	title: string
}

export function folderContextElement(
	path: string
): WorkspaceFolderElement & { deletable: boolean } {
	return { type: 'workspace_folder', path, title: path, deletable: true }
}

/** Whether a folder context path is a user's own items (`u/alice`) rather than a folder. */
export function isUserContextPath(path: string): boolean {
	return /^u\/[^/]+$/.test(path)
}

/** One run of a script or flow, by job id: the chat reads it with `get_run`. */
export interface WorkspaceRunElement {
	type: 'workspace_run'
	/** The job id. */
	id: string
	/** The script or flow that ran. */
	path: string
	title: string
}

export function runContextElement(
	id: string,
	path: string
): WorkspaceRunElement & { deletable: boolean } {
	// The id in the title: two runs of one item are two chips.
	return { type: 'workspace_run', id, path, title: `${path} · run ${id.slice(0, 8)}`, deletable: true }
}

const DIR_PREFIX = dirKey('all', '')

/** Opens every `f/` and `u/` directory of a flat workspace tree (any depth) on a leaf that picks
 * the directory itself, so it is taken as context from inside it, or by searching its path. */
export function withFolderContextLeaves<L>(
	nodes: DrillNode<L>[]
): DrillNode<L | WorkspaceFolderElement>[] {
	return nodes.map((node) => {
		if (node.type !== 'branch') return node
		const children = withFolderContextLeaves(node.children)
		const path = node.key.startsWith(DIR_PREFIX) ? node.key.slice(DIR_PREFIX.length) : ''
		if (!/^[fu]\/./.test(path)) return { ...node, children }
		const leaf: DrillLeaf<WorkspaceFolderElement> = {
			type: 'leaf',
			key: `folder:${path}`,
			label: isUserContextPath(path) ? 'All their items' : 'Whole folder',
			secondary: path,
			icon: Folder,
			data: folderContextElement(path),
			searchableText: path
		}
		return { ...node, children: [leaf, ...children] }
	})
}

/** Message-attached text file, rendered with the shared context badge. Never part
 * of `selectedContext` — constructed at render time from the composer's / a sent
 * message's `files` state (see createAttachedFileContextElement). */
export interface AttachedFileElement {
	type: 'attached_file'
	title: string
	content: string
}

export function createAttachedFileContextElement(
	name: string,
	content: string
): AttachedFileElement {
	return { type: 'attached_file', title: name, content }
}

export type ContextElement = (
	| CodeElement
	| ErrorElement
	| DBElement
	| DiffElement
	| CodePieceElement
	| FlowModuleElement
	| FlowModuleCodePieceElement
	| AppFrontendFileElement
	| AppBackendRunnableElement
	| AppCodeSelectionElement
	| AppDatatableElement
	| AppDomSelectorElement
	| WorkspaceScriptElement
	| WorkspaceFlowElement
	| WorkspaceAppElement
	| WorkspaceFolderElement
	| WorkspaceRunElement
	| AttachedFileElement
) & {
	deletable?: boolean
}

// DOM selector chips can share a display title (two `button.btn` from repeated
// elements), so identify them by (appPath, selector) — keying or removing by
// title would collide, giving repeated chips one Svelte key and deleting them
// together. Other context types stay identified by (type, title).
export function contextElementKey(c: ContextElement): string {
	return c.type === 'app_dom_selector' ? `dom:${c.appPath}:${c.selector}` : `${c.type}:${c.title}`
}

/** An `@`-mentioned workspace item, attached to one message the way a DOM pick
 * is. Membership only: this does not gate on mode, so every caller must. Outside
 * GLOBAL these stay selected as chips the user removes by hand. */
export type MentionContext = Extract<
	ContextElement,
	{
		type: 'workspace_script' | 'workspace_flow' | 'workspace_app' | 'workspace_folder' | 'workspace_run'
	}
>

export function isMentionContext(c: ContextElement): c is MentionContext {
	return (
		c.type === 'workspace_script' ||
		c.type === 'workspace_flow' ||
		c.type === 'workspace_app' ||
		c.type === 'workspace_folder' ||
		c.type === 'workspace_run'
	)
}

export function isSameContextElement(a: ContextElement, b: ContextElement): boolean {
	if (a.type !== b.type) return false
	if (a.type === 'app_dom_selector' && b.type === 'app_dom_selector') {
		return a.selector === b.selector && a.appPath === b.appPath
	}
	return a.title === b.title
}
