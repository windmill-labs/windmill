import { Code, Database, TriangleAlert, Diff } from 'lucide-svelte'
import type { ScriptLang } from '$lib/gen/types.gen'
import { type DBSchema } from '$lib/stores'
import { type Change } from 'diff'

export const ContextIconMap = {
	code: Code,
	error: TriangleAlert,
	db: Database,
	diff: Diff,
	code_piece: Code
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

export type ContextElement = (
	| CodeElement
	| ErrorElement
	| DBElement
	| DiffElement
	| CodePieceElement
	| FlowModuleElement
	| FlowModuleCodePieceElement
) & {
	deletable?: boolean
}
