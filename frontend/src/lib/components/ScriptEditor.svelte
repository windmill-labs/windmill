<script lang="ts">
	import type { Schema } from '$lib/common'
	import { CompletedJob, Job, JobService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { classNames, emptySchema, scriptLangToEditorLang } from '$lib/utils'
	import {
		faCheck,
		faExclamationTriangle,
		faPlay,
		faRotateRight
	} from '@fortawesome/free-solid-svg-icons'
	import { onDestroy, onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import Editor from './Editor.svelte'

	import { inferArgs } from '$lib/infer'

	import type { Preview } from '$lib/gen/models/Preview'

	import SchemaForm from './SchemaForm.svelte'
	import LogPanel from './script_editor/LogPanel.svelte'
	import { HSplitPane, VSplitPane } from 'svelte-split-pane'
	import { faGithub } from '@fortawesome/free-brands-svg-icons'
	import EditorBar from './EditorBar.svelte'
	import Button from './common/button/Button.svelte'

	// Exported
	export let schema: Schema = emptySchema()
	export let code: string
	export let path: string | undefined
	export let lang: Preview.language

	let websocketAlive = { pyright: false, black: false, deno: false }

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
	let lastSave: string | null

	let syncIteration: number = 0
	let ITERATIONS_BEFORE_SLOW_REFRESH = 100

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
		inferSchema()
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

<div class="border-b shadow-sm p-1 pr-4">
	<div class="flex justify-between">
		<EditorBar {editor} {lang} {websocketAlive} />

		<div class="flex divide-x">
			<div>
				<Button
					target="_blank"
					href="https://github.com/windmill-labs/windmill-gh-action-deploy"
					color="light"
					size="xs"
					btnClasses="mr-1"
					startIcon={{
						icon: faGithub
					}}
				>
					Sync from Github
				</Button>
			</div>

			<div>
				<Button
					on:click={() => runPreview()}
					disabled={previewIsLoading}
					btnClasses="w-32 ml-1"
					size="xs"
					startIcon={{
						icon: previewIsLoading ? faRotateRight : faPlay,
						classes: classNames(previewIsLoading ? 'animate-spin' : 'animate-none')
					}}
				>
					{previewIsLoading ? 'Running' : 'Run preview'}
				</Button>
			</div>
		</div>
	</div>
</div>
<div class="flex-1 overflow-auto">
	<HSplitPane leftPaneSize="60%" rightPaneSize="40%" minLeftPaneSize="50px" minRightPaneSize="50px">
		<left slot="left">
			<div class="h-full">
				<div
					class="p-2 h-full"
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
						class="flex flex-1 h-full"
						lang={scriptLangToEditorLang(lang)}
						automaticLayout={true}
					/>
				</div>
			</div>
		</left>
		<right slot="right">
			<div class="h-full">
				<VSplitPane topPanelSize="50%" downPanelSize="50%">
					<top slot="top">
						<div class="h-full overflow-auto">
							<div class="p-4">
								<div class="break-all relative font-sans">
									<div class="items-baseline text-xs text-gray-700 italic hidden md:block">
										<p>
											Move the focus outside of the text editor to recompute the input schema from
											main signature or press Ctrl/Cmd+S
										</p>
										<p class="mt-4">
											{#if isValid}
												<Icon data={faCheck} class="text-green-600 mr-1" scale={0.6} />
												The current preview input matches requirements defined in arguments
											{:else}
												<Icon
													data={faExclamationTriangle}
													class="text-yellow-500 mr-1"
													scale={0.6}
												/>
												The current preview input doesn't match requirements defined in arguments
											{/if}
										</p>
									</div>
									<SchemaForm {schema} bind:args bind:isValid />
								</div>
							</div>
						</div>
					</top>
					<down slot="down">
						<div class="pt-1 h-full overflow-auto">
							<LogPanel
								{path}
								{lang}
								{previewJob}
								{pastPreviews}
								{previewIsLoading}
								bind:lastSave
							/>
						</div>
					</down>
				</VSplitPane>
			</div>
		</right>
	</HSplitPane>
</div>
