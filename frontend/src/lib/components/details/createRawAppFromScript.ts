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

type FieldKind =
	| 'text'
	| 'password'
	| 'resource'
	| 'number'
	| 'boolean'
	| 'enum'
	| 'multienum'
	| 'json'

/** Schema enums are either bare values or `{ value, label }` pairs; the option
 *  submits `value` and displays `label`. */
type EnumOption = { value: string; label: string }

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
	/** Schema `password` flag: masks the input and marks the runnable field
	 *  sensitive, which is what puts it in the policy's `sensitive_inputs`. */
	sensitive: boolean
	/** `format: resource-*`. Without the matching field flag the backend replaces
	 *  a submitted `$res:` reference with a placeholder, so the arg never
	 *  resolves. */
	allowUserResources: boolean
	enumValues: EnumOption[]
	/** `useState(...)` initial value, as TS source. */
	init: string
	/** Value passed to `backend.<runnable>({ ... })`, as TS source. */
	arg: string
}

// Every identifier the generated component references: its own locals and
// setters, plus the globals its body calls. A runnable argument with one of
// these names gets suffixed, so it can neither redeclare nor shadow them (a
// field named `JSON` would otherwise turn `JSON.stringify(result)` into a call
// on the field's value).
export const RESERVED_LOCALS = [
	'React',
	'useState',
	'backend',
	'App',
	'run',
	'e',
	'result',
	'setResult',
	'error',
	'setError',
	'running',
	'setRunning',
	'JSON',
	'Number',
	'String',
	'Error',
	'undefined'
]

// Reserved words can't name a `const`, so an argument called `class` or `new`
// gets suffixed like a collision would be. `eval` and `arguments` are illegal
// binding names in a module (always strict), so they belong here too.
const JS_KEYWORDS = [
	'arguments',
	'await',
	'break',
	'case',
	'catch',
	'class',
	'const',
	'continue',
	'debugger',
	'default',
	'delete',
	'do',
	'else',
	'enum',
	'eval',
	'export',
	'extends',
	'false',
	'finally',
	'for',
	'function',
	'if',
	'implements',
	'import',
	'in',
	'instanceof',
	'interface',
	'let',
	'new',
	'null',
	'package',
	'private',
	'protected',
	'public',
	'return',
	'static',
	'super',
	'switch',
	'this',
	'throw',
	'true',
	'try',
	'typeof',
	'var',
	'void',
	'while',
	'with',
	'yield'
]

/** `u/admin/my_script` -> `my_script`, sanitized to the identifier the raw-app
 *  editor accepts as a runnable id (letters, digits, underscores). */
function runnableIdFromPath(path: string): string {
	const segment = path.split('/').filter(Boolean).pop() ?? ''
	const cleaned = segment.replace(/[^A-Za-z0-9_]/g, '_')
	const id = /^[A-Za-z_]/.test(cleaned) ? cleaned : `run_${cleaned}`
	return cleaned === '' || forbiddenIds.includes(id) ? 'a' : id
}

/** Reserves a state variable and its setter together: an argument named `foo`
 *  claims both `foo` and `setFoo`, so a sibling argument named `setFoo` is
 *  pushed to `setFoo_` instead of redeclaring the first one's setter. */
function allocLocal(base: string, taken: string[]): { local: string; setter: string } {
	const cleaned = base.replace(/[^A-Za-z0-9_$]/g, '_')
	let local = /^[A-Za-z_$]/.test(cleaned) ? cleaned : `_${cleaned}`
	let setter = setterName(local)
	while (taken.includes(local) || taken.includes(setter) || JS_KEYWORDS.includes(local)) {
		local = `${local}_`
		setter = setterName(local)
	}
	taken.push(local, setter)
	return { local, setter }
}

function setterName(local: string): string {
	return `set${local.charAt(0).toUpperCase()}${local.slice(1)}`
}

function isResourceProp(prop: any): boolean {
	return typeof prop?.format === 'string' && prop.format.startsWith('resource-')
}

/** Options live on the property for a scalar enum and on `items` for an array
 *  one; either shape may carry them. */
function enumSource(prop: any): any[] {
	if (Array.isArray(prop?.enum) && prop.enum.length > 0) return prop.enum
	if (Array.isArray(prop?.items?.enum) && prop.items.enum.length > 0) return prop.items.enum
	return []
}

function fieldKind(prop: any): FieldKind {
	if (isResourceProp(prop)) return 'resource'
	if (enumSource(prop).length > 0) {
		// An array-typed enum is a multiselect: `genWmillTs` types it `string[]`,
		// so a scalar `<select>` would generate state the runnable call rejects.
		return prop?.type === 'array' ? 'multienum' : 'enum'
	}
	if (prop?.type === 'boolean') return 'boolean'
	if (prop?.type === 'number' || prop?.type === 'integer') return 'number'
	if (prop?.type === 'string') return prop.password === true ? 'password' : 'text'
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

/** JSX text that would otherwise be swallowed by the parser (`{`, `<`, `}`) or
 *  rewritten by it (`&` starts an entity, so a literal `&amp;` would decode to
 *  `&`) goes through an expression container, which JSX copies verbatim. */
function jsxText(value: string): string {
	return /^[^{}<>&]*$/.test(value) ? value : `{${str(value)}}`
}

/** JSX attribute value: a plain quoted string when it can be, an expression
 *  container otherwise. Entities decode in attributes too, so `&` disqualifies
 *  the plain form — an enum value must reach the runnable unchanged. */
function jsxAttr(value: string): string {
	return /^[^"{}<>&\n]*$/.test(value) ? `"${value}"` : `{${str(value)}}`
}

/** The schema's own description, plus what the control can't convey on its own:
 *  which resource type to point at, and that an object-typed secret is stored
 *  encrypted even though its textarea shows it in the clear (the platform's own
 *  ArgInput says the same rather than masking it). */
function fieldHint(prop: any, kind: FieldKind): string | undefined {
	const own =
		typeof prop.description === 'string' && prop.description !== '' ? prop.description : undefined
	let extra: string | undefined
	if (kind === 'resource') {
		extra = `resource path, e.g. $res:u/user/my_${String(prop.format).slice('resource-'.length)}`
	} else if (kind === 'json' && prop.password === true) {
		extra = 'stored as a secret on submit'
	}
	if (!extra) return own
	return own ? `${own} (${extra})` : extra
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
		const { local, setter } = allocLocal(
			kind === 'number' || kind === 'json' ? `${key}Text` : key,
			taken
		)
		const enumValues: EnumOption[] = enumSource(prop).map((v: any) =>
			v != undefined && typeof v === 'object'
				? { value: String(v.value), label: String(v.label ?? v.value) }
				: { value: String(v), label: String(v) }
		)

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
			// Only a required field is pre-filled with an empty collection: a
			// non-blank optional textarea can never take the omission branch, so an
			// untouched one would override the runnable's own default.
			const seed =
				prop.default != undefined
					? JSON.stringify(prop.default, null, 2)
					: isRequired
						? JSON.stringify(prop.type === 'array' ? [] : {}, null, 2)
						: ''
			init = str(seed)
			arg = isRequired
				? `JSON.parse(${local})`
				: `${local}.trim() === '' ? undefined : JSON.parse(${local})`
		} else if (kind === 'multienum') {
			const defaults = Array.isArray(prop.default) ? prop.default.map((v: any) => String(v)) : []
			// Annotated: `useState([])` alone infers `never[]`.
			init = `[${defaults.map(str).join(', ')}] as string[]`
			arg = isRequired ? local : `${local}.length === 0 ? undefined : ${local}`
		} else if (kind === 'enum') {
			// An optional enum gets a blank option so the runnable's own default
			// stays reachable; a required one always carries a real selection.
			const seed = typeof prop.default === 'string' ? prop.default : ''
			init = str(seed !== '' || !isRequired ? seed : (enumValues[0]?.value ?? ''))
			arg = isRequired ? local : `${local} === '' ? undefined : ${local}`
		} else if (kind === 'resource') {
			// Starts empty, never at a bare `$res:`: an untouched optional field has
			// to read as omitted, and an untouched required one has to trip the
			// browser's `required` check rather than ask the backend to resolve an
			// empty path. The hint carries the expected shape instead.
			init = str(typeof prop.default === 'string' ? prop.default : '')
			arg = isRequired ? local : `${local} === '' ? undefined : ${local}`
		} else {
			init = str(typeof prop.default === 'string' ? prop.default : '')
			// Blank optional text is "unset", so the runnable's own default applies
			// rather than an empty string overriding it.
			arg = isRequired ? local : `${local} === '' ? undefined : ${local}`
		}

		return {
			key,
			kind,
			local,
			setter,
			label: typeof prop.title === 'string' && prop.title !== '' ? prop.title : key,
			description: fieldHint(prop, kind),
			required: isRequired,
			sensitive: prop.password === true,
			allowUserResources: isResourceProp(prop),
			enumValues,
			init,
			arg
		}
	})
}

function fieldInput(field: Field): string {
	// The `*` marker is only decorative without this: the browser has to block
	// the submit, else an empty required field posts '' (or NaN) and fails
	// server-side. Two exemptions: `required` on a checkbox would force it on, and
	// a required single `<select>` always carries a real selection already. A
	// `<select multiple>` can be empty, so it does take the attribute.
	const req =
		field.required && field.kind !== 'boolean' && field.kind !== 'enum'
			? '\n\t\t\t\t\t\trequired'
			: ''
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
						type="number"${req}
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
		case 'enum':
			return `<select
						className="field-input"
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					>
${field.required ? '' : '						<option value=""></option>\n'}${field.enumValues.map((o) => `						<option value=${jsxAttr(o.value)}>${jsxText(o.label)}</option>`).join('\n')}
					</select>`
		case 'multienum':
			// `size` is resolved here and the handler spreads rather than calling
			// `Math.min` / `Array.from`: every global the template names is one more
			// identifier an argument could shadow, so the template names none.
			return `<select
						className="field-input field-multiselect"
						multiple${req}
						size={${Math.min(field.enumValues.length, 6)}}
						value={${field.local}}
						onChange={(e) =>
							${field.setter}([...e.target.selectedOptions].map((o) => o.value))
						}
					>
${field.enumValues.map((o) => `						<option value=${jsxAttr(o.value)}>${jsxText(o.label)}</option>`).join('\n')}
					</select>`
		case 'json':
			return `<textarea
						className="field-input field-textarea"
						rows={4}${req}
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
		case 'resource':
			return `<input
						className="field-input"
						type="text"${req}
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
		case 'password':
			return `<input
						className="field-input"
						type="password"
						autoComplete="off"${req}
						value={${field.local}}
						onChange={(e) => ${field.setter}(e.target.value)}
					/>`
		default:
			return `<input
						className="field-input"
						type="text"${req}
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
			// Shorthand is an own property even for `__proto__`; the `key: value`
			// form is not — there it sets the prototype and the argument never
			// reaches the runnable, so that one key needs a computed key.
			if (f.arg === f.key) return `				${f.key}`
			// `str` may pick a template literal, which is not a legal property
			// name, so keys always take JSON quoting rather than `str`.
			const quoted = JSON.stringify(f.key)
			const key =
				f.key === '__proto__'
					? `[${quoted}]`
					: /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(f.key)
						? f.key
						: quoted
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

const indexCss = `/* The app renders inside an iframe whose host page has its own theme the
   iframe cannot read, so paint an opaque surface here: without one the host's
   background shows through and dark text lands on it unreadable. */
:root {
	color-scheme: light;
	--bg: #ffffff;
	--fg: #18181b;
	--muted: #71717a;
	--border: #d4d4d8;
	--surface: #f4f4f5;
	--accent: #18181b;
	--accent-fg: #ffffff;
	--required: #dc2626;
	--error-bg: #fef2f2;
	--error-fg: #b91c1c;
}

@media (prefers-color-scheme: dark) {
	:root {
		color-scheme: dark;
		--bg: #18181b;
		--fg: #f4f4f5;
		--muted: #a1a1aa;
		--border: #3f3f46;
		--surface: #27272a;
		--accent: #f4f4f5;
		--accent-fg: #18181b;
		--required: #f87171;
		--error-bg: #431a1a;
		--error-fg: #fca5a5;
	}
}

body {
	margin: 0;
	min-height: 100vh;
	background: var(--bg);
	color: var(--fg);
}

.app {
	max-width: 640px;
	margin: 0 auto;
	padding: 24px 16px 48px;
	font-family:
		ui-sans-serif,
		system-ui,
		sans-serif;
	color: var(--fg);
}

.app-title {
	font-size: 1.5rem;
	font-weight: 600;
	margin: 0;
}

.app-subtitle {
	margin: 4px 0 24px;
	font-size: 0.8rem;
	color: var(--muted);
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
	color: var(--required);
	margin-left: 2px;
}

.field-description {
	font-size: 0.75rem;
	color: var(--muted);
}

.field-input {
	border: 1px solid var(--border);
	border-radius: 6px;
	padding: 6px 8px;
	font-size: 0.875rem;
	font-family: inherit;
	background: var(--bg);
	color: var(--fg);
}

.field-multiselect {
	min-height: 72px;
}

.field-textarea {
	font-family: ui-monospace, monospace;
	resize: vertical;
}

.field-checkbox {
	align-self: flex-start;
	width: 16px;
	height: 16px;
	accent-color: var(--accent);
}

.app-empty {
	font-size: 0.875rem;
	color: var(--muted);
	margin: 0;
}

.run-button {
	align-self: flex-start;
	border: none;
	border-radius: 6px;
	background: var(--accent);
	color: var(--accent-fg);
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
	background: var(--error-bg);
	color: var(--error-fg);
}

.app-result {
	background: var(--surface);
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
	// `updateRawAppPolicy` derives the policy's `sensitive_inputs` and
	// `allow_user_resources` from the runnable's fields, so an argument has to be
	// declared here or the secret lands in the job args in the clear / the
	// submitted `$res:` reference is replaced by a placeholder.
	const runnableFields = Object.fromEntries(
		fields
			.filter((f) => f.sensitive || f.allowUserResources)
			.map((f) => [
				f.key,
				{
					type: 'user',
					value: undefined,
					...(f.sensitive ? { sensitive: true } : {}),
					...(f.allowUserResources ? { allowUserResources: true } : {})
				}
			])
	)

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
					fields: runnableFields
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
