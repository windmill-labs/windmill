<script lang="ts">
	import type { Schema } from '$lib/common'
	import { CompletedJob, Job, JobService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { displayDate, emptySchema } from '$lib/utils'
	import {
		faCheck,
		faChevronDown,
		faChevronUp,
		faExclamationTriangle,
		faSpinner,
		faTimes
	} from '@fortawesome/free-solid-svg-icons'
	import { onDestroy, onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'
	import { fade } from 'svelte/transition'
	import Editor from './Editor.svelte'
	import Modal from './Modal.svelte'
	import TableCustom from './TableCustom.svelte'
	import Tooltip from './Tooltip.svelte'

	import { inferArgs } from '$lib/infer'

	// @ts-ignore
	import type { Preview } from '$lib/gen/models/Preview'
	import { Highlight } from 'svelte-highlight'
	import { json, python, typescript } from 'svelte-highlight/languages'
	import { VSplitPane } from 'svelte-split-pane'
	import DisplayResult from './DisplayResult.svelte'
	import EditorBar from './EditorBar.svelte'
	import SchemaForm from './SchemaForm.svelte'

	// Exported
	export let schema: Schema = emptySchema()

	export let code: string
	export let path: string | undefined
	export let lang: Preview.language

	// Control Editor layout
	export let viewPreview = true
	export let previewTab: 'logs' | 'input' | 'output' | 'history' | 'last_save' = 'logs'

	let websocketAlive = { pyright: false, black: false, deno: false }

	let modalViewerTitle: string = ''
	let modalViewerContent: any
	let modalViewerMode: 'logs' | 'result' | 'code' = 'logs'

	// Internal state
	let editor: Editor

	// Preview args input
	let args: Record<string, any> = {}
	let isValid: boolean = true

	// Preview
	let previewIsLoading = false
	let previewIntervalId: NodeJS.Timer
	let previewJob: Job | undefined
	let pastPreviews: CompletedJob[] = []

	let modalViewer: Modal

	let syncIteration: number = 0
	let ITERATIONS_BEFORE_SLOW_REFRESH = 100

	let lastSave: string | null

	$: lastSave = localStorage.getItem(path ?? 'last_save')

	export function getEditor(): Editor {
		return editor
	}

	let div: HTMLElement | null = null

	export async function runPreview(): Promise<void> {
		try {
			if (previewIntervalId) {
				clearInterval(previewIntervalId)
			}
			if (previewIsLoading && previewJob) {
				JobService.cancelQueuedJob({
					workspace: $workspaceStore!,
					id: previewJob.id,
					requestBody: {}
				})
			}
			previewIsLoading = true

			const previewId = await JobService.runScriptPreview({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					content: editor.getCode(),
					args: args,
					language: lang
				}
			})
			previewJob = undefined
			loadPreviewJob(previewId)
			syncIteration = 0
			previewIntervalId = setInterval(() => {
				syncer(previewId)
			}, 500)
			//TODO fetch preview, every x time, until it's completed
		} catch (err) {
			previewIsLoading = false
			throw err
		}
	}

	async function loadPastPreviews(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}

	async function loadPreviewJob(id: string): Promise<void> {
		try {
			if (previewJob && `running` in previewJob) {
				let previewJobUpdates = await JobService.getJobUpdates({
					workspace: $workspaceStore!,
					id,
					running: previewJob.running,
					logOffset: previewJob.logs?.length ?? 0
				})

				if (previewJobUpdates.new_logs) {
					previewJob.logs = (previewJob.logs ?? '').concat(previewJobUpdates.new_logs)
				}
				if ((previewJobUpdates.running ?? false) || (previewJobUpdates.completed ?? false)) {
					previewJob = await JobService.getJob({ workspace: $workspaceStore!, id })
				}
			} else {
				previewJob = await JobService.getJob({ workspace: $workspaceStore!, id })
			}
			if (previewJob?.type === 'CompletedJob') {
				//only CompletedJob has success property
				clearInterval(previewIntervalId)
				previewIsLoading = false
				loadPastPreviews()
			}
			div?.scroll({ top: div?.scrollHeight, behavior: 'smooth' })
		} catch (err) {
			console.error(err)
		}
	}

	async function inferSchema() {
		let isDefault: string[] = []
		Object.entries(args).forEach(([k, v]) => {
			if (schema.properties[k].default == v) {
				isDefault.push(k)
			}
		})
		await inferArgs(lang, editor.getCode(), schema)
		schema = schema

		isDefault
			.filter((key) => schema.properties[key] != undefined)
			.forEach((key) => (args[key] = schema.properties[key].default))
		for (const key of Object.keys(args)) {
			if (schema.properties[key] == undefined) {
				delete args[key]
			}
		}
	}

	function syncer(id: string): void {
		if (syncIteration > ITERATIONS_BEFORE_SLOW_REFRESH) {
			loadPreviewJob(id)
			if (previewIntervalId) {
				clearInterval(previewIntervalId)
				previewIntervalId = setInterval(() => loadPreviewJob(id), 5000)
			}
		} else {
			syncIteration++
			loadPreviewJob(id)
		}
	}

	let syncCode: NodeJS.Timer
	onMount(() => {
		syncCode = setInterval(() => {
			const newCode = editor?.getCode()
			if (newCode && code != newCode) {
				code = editor.getCode()
			}
		}, 3000)
	})
	onDestroy(() => {
		if (editor) {
			code = editor.getCode()
		}
		if (previewIntervalId) {
			clearInterval(previewIntervalId)
		}
		if (syncCode) {
			clearInterval(syncCode)
		}
	})
</script>

<Modal bind:this={modalViewer}>
	<div slot="title">{modalViewerTitle}</div>
	<div slot="content">
		{#if modalViewerMode === 'result'}
			<Highlight language={json} code={JSON.stringify(modalViewerContent, null, 4)} />
		{:else if modalViewerMode === 'logs'}
			<pre class="overflow-x-auto break-all relative h-full m-2 text-xs bg-white shadow-inner">
				{modalViewerContent}
			</pre>
		{:else if modalViewerMode === 'code'}
			{#if lang == 'python3'}
				<Highlight language={python} code={modalViewerContent} />
			{:else if lang == 'deno'}
				<Highlight language={typescript} code={modalViewerContent} />
			{/if}
		{/if}
	</div></Modal
>

<VSplitPane
	topPanelSize={viewPreview ? '75%' : '90%'}
	downPanelSize={viewPreview ? '25%' : '10%'}
	updateCallback={() => {
		if (!viewPreview) {
			viewPreview = true
		}
	}}
>
	<top slot="top">
		<div class="flex flex-col h-full">
			<div class="header">
				<EditorBar {editor} {lang} {websocketAlive} />
			</div>
			<div
				class="flex-1 overflow-hidden border p-2 rounded"
				on:mouseleave={() => {
					code = getEditor().getCode()
					inferSchema()
				}}
			>
				<Editor
					{code}
					bind:websocketAlive
					bind:this={editor}
					cmdEnterAction={() => {
						runPreview()
						viewPreview = true
					}}
					formatAction={async () => {
						code = getEditor().getCode()
						await inferSchema()
						localStorage.setItem(path ?? 'last_save', code)
						lastSave = code
					}}
					on:blur={() => {
						code = getEditor().getCode()
						inferSchema()
					}}
					class="h-full"
					deno={lang == 'deno'}
					automaticLayout={true}
				/>
			</div>
		</div>
	</top>
	<down slot="down" class="flex flex-col h-full">
		<div class="header">
			<div
				class="flex flex-row w-full cursor-pointer h-full"
				on:click={() => {
					viewPreview = !viewPreview
				}}
			>
				<div class="flex flex-row flex-wrap items-baseline">
					<div class="font-base py-0 mr-6 hidden md:block">
						Preview <Tooltip
							><span class="font-normal"
								>Test your script by running a preview, passing inputs as if you were a user</span
							></Tooltip
						>
						<span style="min-width: 15px; display: inline-block;">
							{#if previewIsLoading}
								<span transition:fade>
									<Icon class="animate-spin" data={faSpinner} scale={0.8} />
								</span>
							{/if}
						</span>
					</div>
					<button
						class="font-semibold  my-0 py-0 h-full {previewTab === 'input'
							? 'underline drop-shadow-md'
							: ''}"
						on:click|stopPropagation={() => {
							previewTab = 'input'
							viewPreview = true
							inferSchema()
						}}
					>
						Inputs
					</button>
					<button
						class="font-semibold my-0 py-0 h-full ml-3 {previewTab === 'logs' ? 'underline' : ''}"
						on:click|stopPropagation={() => {
							previewTab = 'logs'
							viewPreview = true
						}}
					>
						Logs
					</button>
					<button
						class="font-semibold my-0 py-0 h-full ml-3 {previewTab === 'output' ? 'underline' : ''}"
						on:click|stopPropagation={() => {
							previewTab = 'output'
							viewPreview = true
						}}
					>
						Result
					</button>
					<button
						class="font-semibold my-0 py-0 h-full ml-3 {previewTab === 'history'
							? 'underline'
							: ''}"
						on:click|stopPropagation={() => {
							if (pastPreviews.length == 0) {
								loadPastPreviews()
							}
							previewTab = 'history'
							viewPreview = true
						}}
					>
						History
					</button>
					<button
						class="font-semibold my-0 py-0 h-full ml-3 {previewTab === 'last_save'
							? 'underline'
							: ''}"
						on:click|stopPropagation={() => {
							previewTab = 'last_save'
							viewPreview = true
						}}
					>
						Local save
					</button>
				</div>
				<div class="flex flex-row-reverse grow">
					<button
						class="mb-1 ml-2"
						on:click|stopPropagation={() => {
							viewPreview = !viewPreview
						}}
						><Icon data={viewPreview ? faChevronDown : faChevronUp} scale={0.7} />
					</button>
					<button
						class="default-button py-px text-xs mx-2 align-middle max-h-8"
						on:click|stopPropagation={() => {
							runPreview()
							viewPreview = true
							previewTab = 'logs'
						}}
						>Run preview
					</button>
					<div class="text-xs text-gray-700 min-w-max hidden md:block mx-2">
						Shortcuts: <Tooltip>
							Cmd/Ctrl+S: autoformat code and overwrite local save <br />
							Cmd/Ctrl+Enter: run preview</Tooltip
						>
					</div>
				</div>
			</div>
		</div>
		<div bind:this={div} class="preview flex-1 p-3">
			{#if previewTab === 'logs'}
				<pre
					class="break-all relative h-full mx-2">{#if previewJob && previewJob.logs}{previewJob.logs}
					{:else if previewIsLoading}Starting preview ...
					{:else}No preview is available yet{/if}
			</pre>
			{:else if previewTab === 'input'}
				<div class="break-all relative h-full font-sans -mt-2">
					<div class="items-baseline text-xs text-gray-700 px-2 ml-8 italic hidden md:block">
						<p>
							Move the focus outside of the text editor to recompute the input schema from main
							signature or press Ctrl/Cmd+S
						</p>
						<p class="">
							{#if isValid}
								<Icon data={faCheck} class="text-green-600 mr-1" scale={0.6} /> The current preview input
								matches requirements defined in arguments
							{:else}
								<Icon data={faExclamationTriangle} class="text-yellow-500 mr-1" scale={0.6} />The
								current preview input doesn't match requirements defined in arguments{/if}
						</p>
					</div>
					<div class="sm:px-8">
						<SchemaForm {schema} bind:args bind:isValid />
					</div>
				</div>
			{:else if previewTab === 'output'}
				<pre class="overflow-x-auto break-all relative h-full">
				{#if previewJob && 'result' in previewJob && previewJob.result}
						<DisplayResult result={previewJob.result} />
					{:else if previewIsLoading}
						Running...
					{:else}
						No output is available yet
					{/if}
			</pre>
			{:else if previewTab === 'last_save'}
				<div class="m-4">
					{#if lastSave}
						<a
							href="#last_save"
							class="text-xs"
							on:click={() => {
								modalViewerContent = lastSave
								modalViewerMode = 'code'
								modalViewer.openModal()
							}}>View last local save for path {path}</a
						>
					{:else}No local save{/if}
				</div>
			{:else if previewTab === 'history'}
				<TableCustom>
					<tr slot="header-row">
						<th class="text-xs">id</th>
						<th class="text-xs">created at</th>
						<th class="text-xs">success</th>
						<th class="text-xs">result</th>
						<th class="text-xs">code</th>
						<th class="text-xs">logs</th>
					</tr>
					<tbody slot="body">
						{#each pastPreviews as { id, created_at, success, result }}
							<tr class="">
								<td class="text-xs">
									<a class="pr-3" href="/run/{id}" target="_blank">{id.substring(30)}</a>
								</td>
								<td class="text-xs">{displayDate(created_at)}</td>
								<td class="text-xs">
									{#if success}
										<Icon class="text-green-600" data={check} scale={0.6} />
									{:else}
										<Icon class="text-red-700" data={faTimes} scale={0.6} />
									{/if}
								</td>
								<td class="text-xs">
									<a
										href="#result"
										class="text-xs"
										on:click={() => {
											modalViewerContent = result
											modalViewerMode = 'result'
											modalViewer.openModal()
										}}
										>{JSON.stringify(result).substring(0, 30)}...
									</a>
								</td>
								<td class="text-xs"
									><a
										href="#code"
										class="text-xs"
										on:click={async () => {
											modalViewerContent = (
												await JobService.getCompletedJob({
													workspace: $workspaceStore ?? 'NO_W',
													id
												})
											).raw_code
											modalViewerMode = 'code'
											modalViewer.openModal()
										}}
										>View code
									</a></td
								>
								<td
									><a
										href="#logs"
										class="text-xs"
										on:click={async () => {
											modalViewerContent = (
												await JobService.getCompletedJob({
													workspace: $workspaceStore ?? 'NO_W',
													id
												})
											).logs
											modalViewerMode = 'logs'
											modalViewer.openModal()
										}}
										>View logs
									</a></td
								>
							</tr>
						{/each}
					</tbody>
				</TableCustom>
			{/if}
		</div>
	</down>
</VSplitPane>

<style>
	.header {
		@apply py-2;
	}

	.preview {
		@apply border;
		@apply overflow-auto;
		@apply bg-white border-l border-r;
		@apply text-xs font-mono;
	}
</style>
