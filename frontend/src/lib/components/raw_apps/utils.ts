import type { ScriptLang } from '../../gen/types.gen'
import type { Schema } from '../../common'
import { schemaToTsType } from '../../schema'
import { isRunnableByName, isRunnableByPath, type RunnableWithFields } from '../apps/inputType'
import type { InlineScript } from '../apps/sharedTypes'

// export type RunnableWithFields = any

type RunnableWithInlineScript = RunnableWithFields & {
	inlineScript?: InlineScript & { language: ScriptLang }
}
export type Runnable = RunnableWithInlineScript | undefined

export type RawApp = {
	files: string[]
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
