import type { ScriptLang } from '../../gen/types.gen'
import type { Schema } from '../../common'
import { schemaToTsType } from '../../schema'
import { isRunnableByName, isRunnableByPath, type RunnableWithFields } from '../apps/inputType'
import type { InlineScript } from '../apps/sharedTypes'
import { deepEqual } from 'fast-equals'

// export type RunnableWithFields = any

type RunnableWithInlineScript = RunnableWithFields & {
	inlineScript?: InlineScript & { language: ScriptLang }
}
export type Runnable = RunnableWithInlineScript | undefined

export type RawApp = {
	files: string[]
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
		const schema = getPathRunnableSchema(runnable)
		if (schema) {
			return schemaToTsType(removeStaticFields(schema, runnable?.fields ?? {}))
		} else {
			return '{}'
		}
	} else {
		return '{}'
	}
}

function getPathRunnableSchema(runnable: Runnable): Schema | undefined {
	if (!isRunnableByPath(runnable)) {
		return undefined
	}

	const schema = runnable.schema as Schema | { schema?: Schema } | undefined
	if (
		schema &&
		typeof schema === 'object' &&
		'schema' in schema &&
		schema.schema &&
		typeof schema.schema === 'object'
	) {
		return schema.schema
	}

	return schema as Schema | undefined
}

function shouldConvertLegacyStaticFields(
	runnable: RunnableWithInlineScript,
	schema: Schema | undefined
): boolean {
	if (!schema?.properties || !runnable.fields) {
		return false
	}

	const fieldEntries = Object.entries(runnable.fields)
	if (fieldEntries.length === 0) {
		return false
	}

	return fieldEntries.every(([key, field]) => {
		const property = schema.properties?.[key]
		return (
			field?.type === 'static' &&
			property !== undefined &&
			deepEqual(field.value, property.default)
		)
	})
}

function normalizeRawAppRunnable(runnable: Runnable): Runnable {
	if (!isRunnableByPath(runnable)) {
		return runnable
	}

	const schema = getPathRunnableSchema(runnable)
	const fields =
		shouldConvertLegacyStaticFields(runnable, schema)
			? Object.fromEntries(
					Object.entries(runnable.fields ?? {}).map(([key, field]) => [
						key,
						{ ...field, type: 'user' as const }
					])
				) as typeof runnable.fields
			: runnable.fields

	if (schema === runnable.schema && fields === runnable.fields) {
		return runnable
	}

	return {
		...runnable,
		schema,
		fields
	}
}

export function normalizeRawAppRunnables(
	runnables: Record<string, Runnable> | undefined
): Record<string, Runnable> {
	return Object.fromEntries(
		Object.entries(runnables ?? {}).map(([key, runnable]) => [
			key,
			normalizeRawAppRunnable(runnable)
		])
	)
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
