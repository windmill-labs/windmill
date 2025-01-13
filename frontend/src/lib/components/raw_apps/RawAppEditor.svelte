<script lang="ts">
	// import { animateTo } from '$lib/components/apps/editor/appUtils'

	// import Editor from '$lib/components/Editor.svelte'
	// import { extToLang } from '$lib/editorUtils'
	// import {
	// 	loadSandpackClient,
	// 	// type BundlerState,
	// 	type SandpackClient
	// } from '@codesandbox/sandpack-client'
	// import { SandpackRuntime } from '@codesandbox/sandpack-client/clients/runtime'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { HiddenRunnable, JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { capitalize, encodeState } from '$lib/utils'

	import { schemaToTsType } from '$lib/schema'
	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { workspaceStore } from '$lib/stores'

	export let initFiles: Record<string, string>
	export let initRunnables: Record<string, HiddenRunnable>
	export let newApp: boolean
	export let policy: Policy
	export let summary = ''
	export let path: string
	export let newPath: string | undefined = undefined
	export let savedApp:
		| {
				value: any
				draft?: any
				path: string
				summary: string
				policy: any
				draft_only?: boolean
				custom_path?: string
		  }
		| undefined = undefined

	export let diffDrawer: DiffDrawer | undefined = undefined
	export let version: number | undefined = undefined

	// let editor: Editor | undefined = undefined

	let runnables = writable(initRunnables)

	const WMILL_TS = '/wmill.ts'

	let files: Record<string, string> | undefined = undefined
	$: $runnables && files && saveFrontendDraft()

	let draftTimeout: NodeJS.Timeout | undefined = undefined
	function saveFrontendDraft() {
		draftTimeout && clearTimeout(draftTimeout)
		draftTimeout = setTimeout(() => {
			try {
				localStorage.setItem(
					path != '' ? `rawapp-${path}` : 'rawapp',
					encodeState({
						files,
						runnables: $runnables
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	let iframe: HTMLIFrameElement | undefined = undefined

	let appPanelSize = 70

	let jobs: string[] = []
	let jobsById: Record<string, JobById> = {}

	function hiddenRunnableToTsType(runnable: HiddenRunnable) {
		if (runnable.type == 'runnableByName') {
			if (runnable.inlineScript?.schema) {
				return schemaToTsType(runnable.inlineScript?.schema)
			} else {
				return '{}'
			}
		}
	}

	function genWmillTs(runnables: Record<string, HiddenRunnable>) {
		return `// THIS FILE IS READ-ONLY
// AND GENERATED AUTOMATICALLY FROM YOUR RUNNABLES
		
${Object.entries(runnables)
	.map(([k, v]) => `export type RunBg${capitalize(k)} = ${hiddenRunnableToTsType(v)}\n`)

	.join('\n')}

export const runBg = {
	${Object.keys(runnables)
		.map((k) => `${k}: (data: RunBg${capitalize(k)}) => Promise<any>`)
		.join(',\n')}
}
		
export const runBgAsync = {
	${Object.keys(runnables)
		.map((k) => `${k}: (data: RunBg${capitalize(k)}) => Promise<string>`)
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
export function waitJob(id: string): Promise<Job> {}

/**
 * Get a job by id and return immediately with the current state of the job
 * @param id
 */
export function getJob(id: string): Promise<Job> {}
`
	}

	function addWmillClientDts(
		files: Record<string, { code: string; readonly?: boolean }>,
		runnables: Record<string, HiddenRunnable>
	) {
		const code = genWmillTs(runnables)
		return {
			...files,
			[WMILL_TS]: {
				readonly: true,
				code
			}
		}
	}

	$: iframe && files && populateFiles()
	$: iframe && runnables && populateRunnables()
	function populateFiles() {
		// files = addWmillClientDts(files, $runnables)
	}

	function populateRunnables() {
		//
		// runnables.set(initRunnables)
	}

	let selectedRunnable: string | undefined = undefined
</script>

<RawAppBackgroundRunner
	workspace={$workspaceStore ?? ''}
	editor
	{iframe}
	bind:jobs
	bind:jobsById
	runnables={$runnables}
	{path}
/>
<div class="max-h-screen overflow-hidden h-screen min-h-0 flex flex-col">
	<RawAppEditorHeader
		bind:jobs
		bind:jobsById
		bind:savedApp
		bind:summary
		on:restore
		on:savedNewAppPath
		{policy}
		{diffDrawer}
		{newApp}
		{newPath}
		appPath={path}
		{files}
		{runnables}
	/>
	<Splitpanes id="o2" horizontal class="grow">
		<Pane bind:size={appPanelSize}>
			<iframe
				bind:this={iframe}
				title="UI builder"
				src="/ui_builder/index.html"
				class="w-full h-full"
			/>
		</Pane>
		<Pane>
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div class="flex h-full w-full">
				<RawAppInlineScriptsPanel appPath={path} bind:selectedRunnable {runnables} />
			</div>
			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
</div>
