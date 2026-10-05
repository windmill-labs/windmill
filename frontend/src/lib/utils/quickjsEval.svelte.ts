/**
 * Runs the JS expressions a schema or flow carries (a field's `showExpr`, a step input's test
 * prefill). They are written by whoever can edit the item but run in the browser of whoever
 * opens it, so they must not run in the page: there they would act with the viewer's session.
 *
 * They run in QuickJS compiled to WebAssembly, a separate engine with no page, storage, cookie or
 * network to reach. Each evaluation gets a fresh runtime, bounded in time and memory. Values cross
 * as JSON both ways.
 */
import type { QuickJSWASMModule } from 'quickjs-emscripten-core'

const TIMEOUT_MS = 1000
// WebAssembly memory never shrinks, so one greedy expression would keep the engine's peak forever.
const MEMORY_LIMIT = 256 * 1024 * 1024

let engine = $state.raw<QuickJSWASMModule>()
let failed = $state(false)
let loading: Promise<unknown> | undefined

/** Resolves once expressions can be evaluated; starts loading the engine on first call. */
export function quickjsReady(): Promise<unknown> {
	loading ??= Promise.all([
		import('quickjs-emscripten-core'),
		import('@jitl/quickjs-wasmfile-release-sync'),
		import('@jitl/quickjs-wasmfile-release-sync/wasm?url')
	])
		.then(([core, variant, wasm]) =>
			core.newQuickJSWASMModuleFromVariant(
				// Outside a browser (tests) the URL is not fetchable; the variant reads its own file.
				import.meta.env.SSR
					? variant.default
					: core.newVariant(variant.default, { wasmLocation: wasm.default })
			)
		)
		.then(
			(m) => (engine = m),
			(e) => {
				console.error('Could not load the expression engine', e)
				failed = true
			}
		)
	return loading
}

/** Whether `evalSandboxed` can answer; reactive. Starts no load: `quickjsReady` does. */
export function quickjsSettled(): boolean {
	return engine !== undefined || failed
}

// Copying into the engine costs about 10ms per MB on the page thread, on every evaluation.
const MAX_TEXT = 1024 * 1024
const MAX_COPY = 5 * 1024 * 1024

const IDENT = '[A-Za-z_$][\\w$]*'
// Only escapes JSON accepts: `literal` reads the match with `JSON.parse`, and `"\x41"` is left to
// the engine.
const STRING = String.raw`(?:"(?:[^"\\\u0000-\u001f]|\\["\\/bfnrt]|\\u[0-9a-fA-F]{4})*"|'(?:[^'\\\u0000-\u001f]|\\['"\\/bfnrt]|\\u[0-9a-fA-F]{4})*')`
const KEY = `\\s*(?:\\.\\s*(${IDENT})|\\[\\s*(${STRING})\\s*\\])`
const ANY_KEY = `\\s*(?:\\.\\s*${IDENT}|\\[\\s*${STRING}\\s*\\])`
const LOOKUP = new RegExp(
	`^\\s*(${IDENT})((?:${ANY_KEY})*)\\s*` +
		`(?:(===|!==|==|!=)\\s*(${STRING}|-?(?:0|[1-9]\\d*)(?:\\.\\d+)?|true|false|null)\\s*)?;?\\s*$`
)

/** A literal `LOOKUP` matched; a single-quoted string is rewritten as JSON first. */
function literal(text: string): unknown {
	if (text[0] !== "'") return JSON.parse(text)
	const body = text
		.slice(1, -1)
		.replace(/\\'|\\.|"/g, (m) => (m === "\\'" ? "'" : m === '"' ? '\\"' : m))
	return JSON.parse(`"${body}"`)
}

/**
 * Answers `name.key["key"]…`, optionally compared to a JSON literal, straight from `scope`: reading
 * data and comparing it to a constant runs no code, so it needs no sandbox and copies nothing.
 * Anything else, including a key that is not the data's own, is left to the engine.
 */
function cheapAnswer(body: string, scope: Record<string, unknown>): { value: unknown } | undefined {
	// `LOOKUP` backtracks quadratically on long malformed input, and runs before the engine's time
	// limit: an author could freeze the viewer's page. Real conditions are far shorter.
	if (body.length > 1000) return undefined
	const m = LOOKUP.exec(body)
	if (!m || !Object.hasOwn(scope, m[1])) return undefined
	let value = scope[m[1]]
	for (const k of m[2].matchAll(new RegExp(KEY, 'g'))) {
		const key = k[1] ?? (literal(k[2]) as string)
		if (value === null || typeof value !== 'object' || !Object.hasOwn(value, key)) return undefined
		value = value[key]
	}
	if (!m[3]) return { value }
	if (value !== null && typeof value === 'object') return undefined
	const other = literal(m[4])
	const equal = m[3].length === 3 ? value === other : value == other
	return { value: m[3].startsWith('=') ? equal : !equal }
}

/** Text over `MAX_TEXT`, such as an uploaded file, crosses as this instead of its content. */
function tooLarge(text: string) {
	return { tooLarge: true, length: text.length, size: `${(text.length / 2 ** 20).toFixed(1)} MB` }
}

/** What is left of `budget` once `value` is copied in; stops counting below zero. */
function remaining(value: unknown, budget: number, placeholders: boolean): number {
	if (typeof value === 'string') {
		return budget - (placeholders && value.length > MAX_TEXT ? 64 : value.length + 2)
	}
	if (value === null || typeof value !== 'object') return budget - 8
	for (const key in value) {
		budget = remaining(value[key], budget - key.length - 4, placeholders)
		if (budget < 0) break
	}
	return budget
}

/** `value` as JSON for the engine, refused past `MAX_COPY`. */
function copyIn(value: unknown, name: string, placeholders: boolean): string {
	if (remaining(value, MAX_COPY, placeholders) < 0) {
		throw new Error(`${name} is over 5 MB, too large to evaluate in the browser: enter it by hand`)
	}
	return (
		JSON.stringify(value, (_, v) =>
			placeholders && typeof v === 'string' && v.length > MAX_TEXT ? tooLarge(v) : v
		) ?? 'undefined'
	)
}

// Each name an object holds becomes a read-only shell that copies an entry in when it is read, as
// the backend's `results` proxy does: a large step result or upload costs only the expressions
// that read it, and what an expression reads is never guessed from its text.
const SHELLS = `"use strict"
// JSON has no undefined, so the host sends it as this word.
const __decode = (json) => (json === 'undefined' ? undefined : JSON.parse(json))
function __shell(name) {
	const keys = JSON.parse(__keys(name))
	if (keys === null) return __decode(__whole(name))
	const loaded = new Map()
	const read = (k) => {
		if (!loaded.has(k)) loaded.set(k, __decode(__entry(name, k)))
		return loaded.get(k)
	}
	return new Proxy({}, {
		get: (_, k) => (typeof k === 'string' && keys.includes(k) ? read(k) : Object.prototype[k]),
		has: (_, k) => keys.includes(k) || k in Object.prototype,
		ownKeys: () => keys,
		getOwnPropertyDescriptor: (_, k) =>
			keys.includes(k) ? { get: () => read(k), enumerable: true, configurable: true } : undefined,
		set: () => false,
		defineProperty: () => false,
		deleteProperty: () => false
	})
}
`

/**
 * Evaluates `expr` with the keys of `scope` as its only names. Throws on any error, and while the
 * engine is loading: read it in an effect, which re-runs once the engine is in. `placeholders`
 * swaps text over 1 MB for its size, for callers that only take the answer's truthiness: a step
 * input would keep the placeholder as its value.
 */
export function evalSandboxed(
	expr: string,
	scope: Record<string, unknown>,
	placeholders = false
): any {
	const body = expr.startsWith('return ') ? expr.slice(7) : expr
	const cheap = cheapAnswer(body, scope)
	if (cheap) return cheap.value
	quickjsReady()
	if (!engine) {
		throw new Error(`The expression engine ${failed ? 'failed to load' : 'is still loading'}`)
	}
	const rt = engine.newRuntime()
	rt.setMemoryLimit(MEMORY_LIMIT)
	const deadline = Date.now() + TIMEOUT_MS
	rt.setInterruptHandler(() => Date.now() > deadline)
	const vm = rt.newContext()
	try {
		const host: Record<string, (...args: string[]) => string> = {
			__keys: (name: string) => {
				const v = scope[name]
				return v && typeof v === 'object' && !Array.isArray(v)
					? JSON.stringify(Object.keys(v))
					: 'null'
			},
			__whole: (name: string) => copyIn(scope[name], name, placeholders),
			__entry: (name: string, key: string) =>
				copyIn((scope[name] as Record<string, unknown>)[key], `${name}.${key}`, placeholders)
		}
		for (const [fnName, fn] of Object.entries(host)) {
			// The interrupt handler runs only between engine steps, not during these copies.
			const handle = vm.newFunction(fnName, (...args) => {
				if (Date.now() > deadline) throw new Error('Interrupted')
				return vm.newString(fn(...args.map((a) => vm.getString(a))))
			})
			vm.setProp(vm.global, fnName, handle)
			handle.dispose()
		}
		const names = Object.keys(scope)
		const code = `${SHELLS}
${names.map((n) => `const ${n} = __shell(${JSON.stringify(n)})`).join('\n')}
;(() => {
return ${body}
})()`
		const result = vm.unwrapResult(vm.evalCode(code))
		try {
			return vm.dump(result)
		} finally {
			result.dispose()
		}
	} finally {
		vm.dispose()
		rt.dispose()
	}
}

/**
 * Whether a field with `expr` as its `showExpr` is shown for `args`, or undefined while the engine
 * loads. Undefined means decide nothing: hiding a field deletes its value.
 */
export function computeShow(expr: string | undefined, args: any): boolean | undefined {
	if (!expr) return true
	const scope = { fields: args ?? {} }
	const cheap = cheapAnswer(expr.startsWith('return ') ? expr.slice(7) : expr, scope)
	if (cheap) return Boolean(cheap.value)
	quickjsReady()
	if (!quickjsSettled()) return undefined
	try {
		return Boolean(evalSandboxed(expr, scope, true))
	} catch (e) {
		console.error(`Impossible to eval ${expr}:`, e)
		return true
	}
}
