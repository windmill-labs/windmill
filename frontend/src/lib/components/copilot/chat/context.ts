import {
	Code,
	Database,
	TriangleAlert,
	Diff,
	FileCode,
	Code2,
	TextSelect,
	Table2
} from 'lucide-svelte'
import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
import type { ScriptLang, Script, OpenFlow } from '$lib/gen/types.gen'
import { type DBSchema } from '$lib/stores'
import { type Change } from 'diff'
import type { BackendRunnable } from './app/core'

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
	workspace_script: Code2,
	workspace_flow: BarsStaggered
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
	/** The table columns: column_name -> compact_type */
	columns: Record<string, string>
}

/** Workspace script context element — reference to a script in the workspace */
export interface WorkspaceScriptElement
	extends Pick<Script, 'path' | 'summary' | 'language' | 'content' | 'schema'> {
	type: 'workspace_script'
	title: string
}

/** Workspace flow context element — reference to a flow in the workspace */
export interface WorkspaceFlowElement extends Pick<OpenFlow, 'summary' | 'schema'> {
	type: 'workspace_flow'
	path: string
	title: string
	description: string
	/** Full flow value, JSON-stringified and possibly truncated */
	value: string
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
	| WorkspaceScriptElement
	| WorkspaceFlowElement
) & {
	deletable?: boolean
}
