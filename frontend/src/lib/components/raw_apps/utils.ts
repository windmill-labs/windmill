import type { Schema } from '$lib/common'
import { schemaToTsType } from '$lib/schema'
import { capitalize } from '$lib/utils'
import type { HiddenRunnable } from '../apps/types'

export type RawApp = {
	files: string[]
}

export function htmlContent(workspace: string, version: number, ctx: any) {
	return `
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="UTF-8" />
                <title>App Preview</title>
                <link rel="stylesheet" href="/api/w/${workspace}/apps/get_data/v/${version}.css" />
				<script>
					window.ctx = ${ctx ? JSON.stringify(ctx) : 'undefined'}
				</script>
            </head>
            <body>
                <div id="root"></div>
                <script src="/api/w/${workspace}/apps/get_data/v/${version}.js"></script>
            </body>
        </html>
    `
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

function hiddenRunnableToTsType(runnable: HiddenRunnable) {
	if (runnable.type == 'runnableByName') {
		if (runnable.inlineScript?.schema) {
			return schemaToTsType(removeStaticFields(runnable.inlineScript?.schema, runnable.fields))
		} else {
			return '{}'
		}
	} else if (runnable.type == 'runnableByPath') {
		return schemaToTsType(removeStaticFields(runnable.schema, runnable.fields))
	} else {
		return '{}'
	}
}

export function genWmillTs(runnables: Record<string, HiddenRunnable>) {
	return `// THIS FILE IS READ-ONLY
// AND GENERATED AUTOMATICALLY FROM YOUR RUNNABLES
	
${Object.entries(runnables)
	.map(([k, v]) => `export type RunBg${capitalize(k)} = ${hiddenRunnableToTsType(v)}\n`)

	.join('\n')}

export const runBg = {
${Object.keys(runnables)
	.map((k) => `  ${k}: null as unknown as (data: RunBg${capitalize(k)}) => Promise<any>`)
	.join(',\n')}
}
	
export const runBgAsync = {
${Object.keys(runnables)
	.map((k) => `  ${k}: null as unknown as (data: RunBg${capitalize(k)}) => Promise<string>`)
	.join(',\n')}
}
	

export type Job = {
	type: 'QueuedJob' | 'CompletedJob'
	id: string
	created_at: number
	started_at: number | undefined
	duration_ms: number
	success: boolean
	args: any
	result: any
}

/**
* Execute a job and wait for it to complete and return the completed job
* @param id
*/
// @ts-ignore
export function waitJob(id: string): Promise<Job> {
	// implementation passed when bundling/deploying
	return null as unknown as Promise<Job>
}

/**
* Get a job by id and return immediately with the current state of the job
* @param id
*/
// @ts-ignore
export function getJob(id: string): Promise<Job> {
	// implementation passed when bundling/deploying
	return null as unknown as Promise<Job>
}
`
}
