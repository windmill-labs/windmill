import type { ScriptLang } from '../../gen/types.gen'
import type { Schema } from '../../common'
import { schemaToTsType } from '../../schema'
import { isRunnableByName, isRunnableByPath, type RunnableWithFields } from '../apps/inputType'
import type { InlineScript } from '../apps/sharedTypes'
import { stateSnapshot } from '$lib/svelte5Utils.svelte'
import { appSourceToDraftValue, normalizeRawAppData } from './rawAppDraftValue'

// export type RunnableWithFields = any

type RunnableWithInlineScript = RunnableWithFields & {
	inlineScript?: InlineScript & { language: ScriptLang }
}
export type Runnable = RunnableWithInlineScript | undefined

export type RawApp = {
	files: string[]
}

// Server-managed columns the deployed app row (getAppByPath) carries but the
// editor's current value never does — leaving them in renders as spurious diff.
const RAW_APP_DEPLOYED_METADATA_KEYS = [
	'raw_app',
	'id',
	'created_at',
	'created_by',
	'versions',
	'extra_perms',
	'is_draft'
] as const

/**
 * Normalize a raw-app value before a deployed-vs-current diff (or unsaved-change
 * comparison). Three sources of spurious post-deploy diff:
 *  - the deployed row carries server-managed columns (`raw_app`, timestamps, …)
 *    that the editor's current value lacks;
 *  - inline-script `lock`s are recomputed server-side at every deploy and the
 *    editor clears them on edit, so the editor value and the freshly deployed
 *    one always diverge on `lock` even though the user changed nothing there;
 *  - the deployed value omits an empty `data` while the editor always carries
 *    the default `{ tables: [] }`, so even an untouched app reads as changed.
 * All three must be neutralized symmetrically on both sides. Returns a deep
 * clone; never mutates the input (the current side is live editor state).
 */
export function stripRawAppDiffNoise<T extends Record<string, any>>(value: T): T {
	const cloned = structuredClone(stateSnapshot(value)) as Record<string, any>
	for (const key of RAW_APP_DEPLOYED_METADATA_KEYS) {
		delete cloned[key]
	}
	// Runnables/data live under `.value` on a deployed row and on the editor's
	// diff value alike, but a flat draft shape carries them top-level.
	const source = cloned.value ?? cloned
	const runnables = source.runnables
	if (runnables && typeof runnables === 'object') {
		for (const k of Object.keys(runnables)) {
			const inlineScript = runnables[k]?.inlineScript
			if (inlineScript && inlineScript.lock != undefined) {
				inlineScript.lock = undefined
			}
		}
	}
	// Canonicalize `data` so an absent and a default-empty `data` compare equal.
	source.data = normalizeRawAppData(source)
	return cloned as T
}

/**
 * Canonical raw-app value for diffing a *draft* against a *deployed* row. On top
 * of the noise stripped by stripRawAppDiffNoise, the two also differ in shape: a
 * deployed row nests its source under `value`, whereas a draft carries
 * `files`/`runnables`/`data` at the top level. `appSourceToDraftValue` collapses
 * both onto the same flat field set first. Use this for the session/compare
 * draft diff so it matches the editor's Diff button (which shares
 * stripRawAppDiffNoise).
 */
export function canonicalRawAppDiffValue(source: Record<string, any>) {
	return stripRawAppDiffNoise(appSourceToDraftValue(source))
}

export type RawAppRuntimeLogLevel = 'log' | 'info' | 'warn' | 'error' | 'debug'
export type RawAppRuntimeLogEntry = {
	level: RawAppRuntimeLogLevel
	message: string
	ts: number
}
export type RawAppRuntimeLogRequester = (
	limit: number
) => Promise<RawAppRuntimeLogEntry[] | undefined>

const RAW_APP_RUNTIME_LOG_LEVELS = new Set<RawAppRuntimeLogLevel>([
	'log',
	'info',
	'warn',
	'error',
	'debug'
])

function isRawAppRuntimeLogLevel(level: unknown): level is RawAppRuntimeLogLevel {
	return typeof level === 'string' && RAW_APP_RUNTIME_LOG_LEVELS.has(level as RawAppRuntimeLogLevel)
}

function isValidRuntimeLogTimestamp(ts: unknown): ts is number {
	return typeof ts === 'number' && Number.isFinite(ts) && !Number.isNaN(new Date(ts).getTime())
}

export function normalizeRawAppRuntimeLogs(logs: unknown): RawAppRuntimeLogEntry[] {
	if (!Array.isArray(logs)) return []
	return logs.flatMap((entry) => {
		if (!entry || typeof entry !== 'object') return []
		const { level, message, ts } = entry as Record<string, unknown>
		if (
			!isRawAppRuntimeLogLevel(level) ||
			typeof message !== 'string' ||
			!isValidRuntimeLogTimestamp(ts)
		)
			return []
		return [{ level, message, ts }]
	})
}

export function formatRuntimeLogsForChat(entries: RawAppRuntimeLogEntry[]): string {
	const lines = entries.map((e) => {
		const date = new Date(e.ts)
		const time = Number.isNaN(date.getTime()) ? '--:--:--' : date.toISOString().slice(11, 23)
		return `[${time}] ${e.level.toUpperCase()}: ${e.message}`
	})
	return lines.join('\n')
}

export type RawAppRunSummary = {
	job_id: string
	component: string
	status: 'running' | 'completed'
	created_at?: number
	started_at?: number
	duration_ms?: number
}
export type RawAppRunsProvider = () => RawAppRunSummary[] | undefined

export function formatAppRunsForChat(runs: RawAppRunSummary[]): string {
	return JSON.stringify(runs, null, 2)
}

// The sandboxed (isolated) raw-app wrapper is generated server-side and served as
// a sandboxed, opaque-origin document (see `get_raw_app_data` in the backend
// `apps.rs`, WIN-2006) — a blob: URL cannot carry the `CSP: sandbox` response
// header that enforces isolation, so the wrapper must come from the backend.
//
// The function below is used ONLY for the unsandboxed path (the default — the
// publisher did not opt into sandbox isolation). It is loaded as a blob: URL —
// same-origin with the SPA — so, with `allow-same-origin`, the bundle runs with
// the viewer's full session. Crucially this is an in-memory blob, not a
// real-origin endpoint, so it is not a URL an attacker can navigate a logged-in
// victim to in order to gain isolation-bypassing access — the backend `.html`
// document stays sandboxed whenever the publisher did opt in.
export function unsandboxedRawAppHtml(
	workspace: string,
	secret: string,
	ctx: any,
	baseUrl: string,
	initialHash: string
) {
	return `<!DOCTYPE html>
<html>
<head>
	<meta charset="UTF-8" />
	<title>App</title>
	<link rel="stylesheet" href="${baseUrl}/api/w/${workspace}/apps_u/get_data/v/${secret}.css" />
	<script>
		window.ctx = ${ctx ? JSON.stringify(ctx) : 'undefined'};
		(function () {
			// Keep the parent URL hash in sync for shareable URLs.
			function notifyParent() {
				try {
					if (window.parent !== window) {
						window.parent.postMessage({ type: 'windmill:hashchange', hash: window.location.hash }, '*');
					}
				} catch (_) {}
			}
			var initialHash = ${JSON.stringify(initialHash)};
			if (initialHash && initialHash !== '#' && !window.location.hash) {
				try { history.replaceState(null, '', initialHash); } catch (_) {}
			}
			window.addEventListener('hashchange', notifyParent);
			var _ps = history.pushState, _rs = history.replaceState;
			history.pushState = function () { _ps.apply(this, arguments); notifyParent(); };
			history.replaceState = function () { _rs.apply(this, arguments); notifyParent(); };
			setTimeout(notifyParent, 0);
		})();
	</script>
</head>
<body>
	<div id="root"></div>
	<script src="${baseUrl}/api/w/${workspace}/apps_u/get_data/v/${secret}.js"></script>
</body>
</html>`
}

function removeStaticFields(schema: Schema, fields: Record<string, { type: string }>): Schema {
	const staticFields = Object.keys(fields).filter((k) => fields[k].type == 'static')
	return {
		...schema,
		properties: {
			...Object.fromEntries(
				Object.entries(schema.properties ?? {}).filter(([k]) => !staticFields.includes(k))
			)
		}
	}
}

function hiddenRunnableToTsType(runnable: Runnable) {
	if (isRunnableByName(runnable)) {
		if (runnable?.inlineScript?.schema) {
			return schemaToTsType(
				removeStaticFields(runnable?.inlineScript?.schema, runnable?.fields ?? {})
			)
		} else {
			return '{}'
		}
	} else if (isRunnableByPath(runnable)) {
		if (runnable?.schema) {
			return schemaToTsType(removeStaticFields(runnable.schema as Schema, runnable?.fields ?? {}))
		} else {
			return '{}'
		}
	} else {
		return '{}'
	}
}

export function genWmillTs(runnables: Record<string, Runnable>) {
	return `// THIS FILE IS READ-ONLY
// AND GENERATED AUTOMATICALLY FROM YOUR RUNNABLES

export declare const backend: {
${Object.entries(runnables)
	.map(([k, v]) => `  ${k}: (args: ${hiddenRunnableToTsType(v)}) => Promise<any>;`)
	.join('\n')}
};

export declare const backendAsync: {
${Object.entries(runnables)
	.map(([k, v]) => `  ${k}: (args: ${hiddenRunnableToTsType(v)}) => Promise<string>;`)
	.join('\n')}
};

export type Job = {
  type: "QueuedJob" | "CompletedJob";
  id: string;
  created_at: number;
  started_at: number | undefined;
  duration_ms: number;
  success: boolean;
  args: any;
  result: any;
};

/**
 * Execute a job and wait for it to complete and return the completed job
 * @param id
 */
export declare function waitJob(id: string): Promise<Job>;

/**
 * Get a job by id and return immediately with the current state of the job
 * @param id
 */
export declare function getJob(id: string): Promise<Job>;

export type StreamUpdate = {
  new_result_stream?: string;
  stream_offset?: number;
};

/**
 * Stream job results using SSE. Calls onUpdate for each stream update,
 * and resolves with the final result when the job completes.
 * @param id - The job ID to stream
 * @param onUpdate - Optional callback for stream updates with new_result_stream data
 * @returns Promise that resolves with the final job result
 */
export declare function streamJob(id: string, onUpdate?: (data: StreamUpdate) => void): Promise<any>;
`
}
