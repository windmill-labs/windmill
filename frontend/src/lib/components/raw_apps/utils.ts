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

export function formatRuntimeLogsForChat(entries: RawAppRuntimeLogEntry[]): string {
	const lines = entries.map((e) => {
		const time = Number.isFinite(e.ts) ? new Date(e.ts).toISOString().slice(11, 23) : '--:--:--'
		return `[${time}] ${e.level.toUpperCase()}: ${e.message}`
	})
	return [
		`Runtime logs from the raw app preview (${entries.length}, oldest first).`,
		'Use these browser-side logs to diagnose frontend runtime errors.',
		'If a backend.<id>() call failed or returned unexpected data, call list_app_runs, pick the relevant job_id, then call get_job_logs with id=job_id.',
		'',
		lines.join('\n')
	].join('\n')
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
	return [
		`Backend runs triggered by the raw app preview (${runs.length}, newest first).`,
		'Use the component field to match a frontend backend.<component>() call.',
		'Next step: choose the relevant job_id and call get_job_logs with id=job_id to inspect server-side logs.',
		'',
		JSON.stringify(runs, null, 2)
	].join('\n')
}

export function htmlContent(
	workspace: string,
	secret: string | undefined,
	ctx: any,
	baseUrl: string = '',
	initialHash: string = ''
) {
	return `<!DOCTYPE html>
<html>
<head>
	<meta charset="UTF-8" />
	<title>App Preview</title>
	<link rel="stylesheet" href="${baseUrl}/api/w/${workspace}/apps_u/get_data/v/${secret}.css" />
	<script>
		window.ctx = ${ctx ? JSON.stringify(ctx) : 'undefined'};

		// Sync hash with parent window for shareable URLs
		(function() {
			// Set initial hash from parent URL
			var initialHash = ${JSON.stringify(initialHash)};
			if (initialHash && initialHash !== '#' && !window.location.hash) {
				history.replaceState(null, '', initialHash);
			}

			// Notify parent when hash changes
			function notifyParent() {
				var hash = window.location.hash;
				console.log('[HashSync] notifyParent called, hash:', hash);
				if (window.parent !== window) {
					window.parent.postMessage({
						type: 'windmill:hashchange',
						hash: hash
					}, '*');
				}
			}

			// Listen for hash changes
			window.addEventListener('hashchange', function() {
				console.log('[HashSync] hashchange event');
				notifyParent();
			});

			// Also notify on pushState/replaceState
			var originalPushState = history.pushState;
			var originalReplaceState = history.replaceState;

			history.pushState = function() {
				console.log('[HashSync] pushState called with:', arguments[2]);
				originalPushState.apply(this, arguments);
				notifyParent();
			};

			history.replaceState = function() {
				console.log('[HashSync] replaceState called with:', arguments[2]);
				originalReplaceState.apply(this, arguments);
				notifyParent();
			};

			// Notify parent of initial hash after load
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
