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

export type ContextElement = CodeElement | ErrorElement | DBElement | DiffElement | CodePieceElement
