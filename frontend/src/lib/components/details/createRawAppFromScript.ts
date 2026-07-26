import { forbiddenIds } from '$lib/components/flows/idUtils'
import { react19Template } from '$lib/components/raw_apps/templates'

/** Shape the `/apps_raw/add` import handoff consumes (`$importStore` /
 *  `sessionStorage.rawAppImport`). */
export type RawAppImport = {
	summary: string
	value: {
		files: Record<string, string>
		runnables: Record<string, any>
	}
}

type FieldKind = 'text' | 'number' | 'boolean' | 'enum' | 'json'

type Field = {
	/** Schema property name, i.e. the runnable argument key. */
	key: string
	kind: FieldKind
	/** React state variable holding this field's input value. */
	local: string
	setter: string
	label: string
	description: string | undefined
	required: boolean
	enumValues: string[]
	/** `useState(...)` initial value, as TS source. */
	init: string
	/** Value passed to `backend.<runnable>({ ... })`, as TS source. */
	arg: string
}

// Locals the generated component already defines; a runnable argument with one
// of these names gets suffixed so it can't shadow them.
const RESERVED_LOCALS = ['React', 'useState', 'backend', 'App', 'run', 'result', 'error', 'running']

/** `u/admin/my_script` -> `my_script`, sanitized to the identifier the raw-app
 *  editor accepts as a runnable id (letters, digits, underscores). */
function runnableIdFromPath(path: string): string {
	const segment = path.split('/').filter(Boolean).pop() ?? ''
	const cleaned = segment.replace(/[^A-Za-z0-9_]/g, '_')
	const id = /^[A-Za-z_]/.test(cleaned) ? cleaned : `run_${cleaned}`
	return cleaned === '' || forbiddenIds.includes(id) ? 'a' : id
}

function toLocal(key: string, taken: string[]): string {
	const cleaned = key.replace(/[^A-Za-z0-9_$]/g, '_')
	let local = /^[A-Za-z_$]/.test(cleaned) ? cleaned : `_${cleaned}`
	while (taken.includes(local)) {
		local = `${local}_`
	}
	return local
}

function fieldKind(prop: any): FieldKind {
	if (Array.isArray(prop?.enum) && prop.enum.length > 0) return 'enum'
	if (prop?.type === 'boolean') return 'boolean'
	if (prop?.type === 'number' || prop?.type === 'integer') return 'number'
	if (prop?.type === 'string') return 'text'
	return 'json'
}

// Newline excluded: a template literal can carry it verbatim.
// eslint-disable-next-line no-control-regex
const CONTROL_CHARS = /[\u0000-\u0009\u000b-\u001f\u007f]/

/** TS string literal. Prefers the quoting that keeps the generated source
 *  readable, falling back to `JSON.stringify`'s escaping. */
function str(value: string): string {
	if (value.includes('\\') || value.includes('\r') || CONTROL_CHARS.test(value)) {
		return JSON.stringify(value)
	}
	if (!value.includes("'") && !value.includes('\n')) return `'${value}'`
	if (!value.includes('`') && !value.includes('${')) return `\`${value}\``
	return JSON.stringify(value)
}

/** JSX text that would otherwise be swallowed by the parser (`{`, `<`, `}`)
 *  goes through an expression container. */
function jsxText(value: string): string {
	return /^[^{}<>]*$/.test(value) ? value : `{${str(value)}}`
}

/** JSX attribute value: a plain quoted string when it can be, an expression
 *  container otherwise. */
function jsxAttr(value: string): string {
	return /^[^"{}<>\n]*$/.test(value) ? `"${value}"` : `{${str(value)}}`
}

function toFields(schema: Record<string, any> | undefined): Field[] {
	const properties: Record<string, any> = schema?.properties ?? {}
	const order: string[] = Array.isArray(schema?.order) ? schema.order : []
	const keys = [
		...order.filter((k) => k in properties),
		...Object.keys(properties).filter((k) => !order.includes(k))
	]
	const required: string[] = Array.isArray(schema?.required) ? schema.required : []
	const taken = [...RESERVED_LOCALS]

	return keys.map((key) => {
		const prop = properties[key] ?? {}
		const kind = fieldKind(prop)
		const isRequired = required.includes(key)
		const local = toLocal(kind === 'number' || kind === 'json' ? `${key}Text` : key, taken)
		taken.push(local)
		const enumValues = (Array.isArray(prop.enum) ? prop.enum : []).map((v: any) => String(v))

		let init: string
		let arg: string
		if (kind === 'boolean') {
			init = prop.default === true ? 'true' : 'false'
			arg = local
		} else if (kind === 'number') {
			init = str(prop.default != undefined ? String(prop.default) : '')
			// An emptied input means "unset", which only type-checks when the
			// argument is optional.
			arg = isRequired ? `Number(${local})` : `${local} === '' ? undefined : Number(${local})`
		} else if (kind === 'json') {
			const fallback = prop.type === 'array' ? [] : {}
			init = str(JSON.stringify(prop.default ?? fallback, null, 2))
			arg = isRequired
				? `JSON.parse(${local})`
				: `${local}.trim() === '' ? undefined : JSON.parse(${local})`
		} else {
			const fallback = kind === 'enum' ? (enumValues[0] ?? '') : ''
			init = str(typeof prop.default === 'string' ? prop.default : fallback)
			arg = local
		}

		return {
			key,
			kind,
			local,
			setter: `set${local.charAt(0).toUpperCase()}${local.slice(1)}`,
			label: typeof prop.title === 'string' && prop.title !== '' ? prop.title : key,
			description:
				typeof prop.description === 'string' && prop.description !== ''
					? prop.description
					: undefined,
			required: isRequired,
			enumValues,
			init,
			arg
		}
	})
}

function fieldInput(field: Field): string {
	switch (field.kind) {
		case 'boolean':
			return `<input
						className="field-checkbox"
						type="checkbox"
						checked={${field.local}}
						onChange={(e) => ${field.setter}(e.target.checked)}
					/>`
		case 'number':
			return `<input
						className="field-input"
						type="number"
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
		case 'enum':
			return `<select
						className="field-input"
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					>
${field.enumValues.map((v) => `						<option value=${jsxAttr(v)}>${jsxText(v)}</option>`).join('\n')}
					</select>`
		case 'json':
			return `<textarea
						className="field-input field-textarea"
						rows={4}
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
		default:
			return `<input
						className="field-input"
						type="text"
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
	}
}

function fieldJsx(field: Field): string {
	const label = `<span className="field-label">
						${jsxText(field.label)}${field.required ? '\n\t\t\t\t\t\t<span className="field-required">*</span>' : ''}
					</span>`
	const description = field.description
		? `\n					<span className="field-description">${jsxText(field.description)}</span>`
		: ''
	return `				<label className="field">
					${label}${description}
					${fieldInput(field)}
				</label>`
}

function generateAppTsx(opts: {
	runnableId: string
	fields: Field[]
	title: string
	subtitle: string
}): string {
	const { runnableId, fields, title, subtitle } = opts
	const states = fields
		.map((f) => `	const [${f.local}, ${f.setter}] = useState(${f.init})`)
		.join('\n')
	const args = fields
		.map((f) => {
			if (f.arg === f.key) return `				${f.key}`
			const key = /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(f.key) ? f.key : str(f.key)
			return `				${key}: ${f.arg}`
		})
		.join(',\n')
	const callArgs = fields.length === 0 ? '{}' : `{\n${args}\n			}`
	const form =
		fields.length === 0
			? '				<p className="app-empty">This runnable takes no arguments.</p>\n'
			: fields.map(fieldJsx).join('\n') + '\n'

	return `import React, { useState } from 'react'
import { backend } from './wmill'
import './index.css'

const App = () => {
${states}${states === '' ? '' : '\n'}	const [result, setResult] = useState(undefined as any)
	const [error, setError] = useState(undefined as string | undefined)
	const [running, setRunning] = useState(false)

	async function run() {
		setRunning(true)
		setError(undefined)
		try {
			setResult(await backend.${runnableId}(${callArgs}))
		} catch (e) {
			setResult(undefined)
			setError(e instanceof Error ? e.message : String(e))
		}
		setRunning(false)
	}

	return (
		<main className="app">
			<h1 className="app-title">${jsxText(title)}</h1>
			<p className="app-subtitle">${jsxText(subtitle)}</p>
			<form
				className="app-form"
				onSubmit={(e) => {
					e.preventDefault()
					run()
				}}
			>
${form}				<button className="run-button" type="submit" disabled={running}>
					{running ? 'Running...' : 'Run'}
				</button>
			</form>
			{error !== undefined && <pre className="app-error">{error}</pre>}
			{result !== undefined && <pre className="app-result">{JSON.stringify(result, null, 2)}</pre>}
		</main>
	)
}

export default App
`
}

const indexCss = `.app {
	max-width: 640px;
	margin: 0 auto;
	padding: 24px 16px 48px;
	font-family:
		ui-sans-serif,
		system-ui,
		sans-serif;
	color: #18181b;
}

.app-title {
	font-size: 1.5rem;
	font-weight: 600;
	margin: 0;
}

.app-subtitle {
	margin: 4px 0 24px;
	font-size: 0.8rem;
	color: #71717a;
}

.app-form {
	display: flex;
	flex-direction: column;
	gap: 16px;
}

.field {
	display: flex;
	flex-direction: column;
	gap: 4px;
}

.field-label {
	font-size: 0.8rem;
	font-weight: 500;
}

.field-required {
	color: #dc2626;
	margin-left: 2px;
}

.field-description {
	font-size: 0.75rem;
	color: #71717a;
}

.field-input {
	border: 1px solid #d4d4d8;
	border-radius: 6px;
	padding: 6px 8px;
	font-size: 0.875rem;
	font-family: inherit;
}

.field-textarea {
	font-family: ui-monospace, monospace;
	resize: vertical;
}

.field-checkbox {
	align-self: flex-start;
	width: 16px;
	height: 16px;
}

.app-empty {
	font-size: 0.875rem;
	color: #71717a;
	margin: 0;
}

.run-button {
	align-self: flex-start;
	border: none;
	border-radius: 6px;
	background: #18181b;
	color: white;
	padding: 8px 16px;
	font-size: 0.875rem;
	cursor: pointer;
}

.run-button:disabled {
	opacity: 0.6;
	cursor: default;
}

.app-error,
.app-result {
	margin-top: 24px;
	padding: 12px;
	border-radius: 6px;
	font-size: 0.8rem;
	white-space: pre-wrap;
	overflow-x: auto;
}

.app-error {
	background: #fef2f2;
	color: #b91c1c;
}

.app-result {
	background: #f4f4f5;
}
`

function createRawApp(opts: {
	path: string
	summary: string | undefined
	schema: Record<string, any> | undefined
	runType: 'script' | 'flow'
}): RawAppImport {
	const { path, summary, schema, runType } = opts
	const runnableId = runnableIdFromPath(path)
	const fields = toFields(schema)
	const title = summary && summary !== '' ? summary : path

	return {
		summary: title,
		value: {
			files: {
				...react19Template,
				'/App.tsx': generateAppTsx({
					runnableId,
					fields,
					title,
					subtitle: `${runType} · ${path}`
				}),
				'/index.css': indexCss
			},
			runnables: {
				[runnableId]: {
					name: path,
					type: 'path',
					runType,
					path,
					schema: schema ?? {},
					fields: {}
				}
			}
		}
	}
}

/** React app scaffold that runs `path` from a form built off its schema. */
export function createRawAppFromScript(
	path: string,
	summary: string | undefined,
	schema: Record<string, any> | undefined
): RawAppImport {
	return createRawApp({ path, summary, schema, runType: 'script' })
}

export function createRawAppFromFlow(
	path: string,
	summary: string | undefined,
	schema: Record<string, any> | undefined
): RawAppImport {
	return createRawApp({ path, summary, schema, runType: 'flow' })
}
