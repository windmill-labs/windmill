<script lang="ts">
	import type { Schema } from '$lib/common'
	import { CompletedJob, Job, JobService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, scriptLangToEditorLang } from '$lib/utils'
	import {
		faCheck,
		faExclamationTriangle,
		faPlay,
		faRotateRight
	} from '@fortawesome/free-solid-svg-icons'
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
	import TestJobLoader from './TestJobLoader.svelte'
	import { onMount } from 'svelte'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'

	// Exported
	export let schema: Schema = emptySchema()
	export let code: string
	export let path: string | undefined
	export let lang: Preview.language

	let websocketAlive = { pyright: false, black: false, deno: false }

	// Internal state
	let editor: Editor

	let testJobLoader: TestJobLoader

	// Test args input
	let args: Record<string, any> = {}
	let isValid: boolean = true

	// Test
	let testIsLoading = false
	let testJob: Job | undefined
	let pastPreviews: CompletedJob[] = []
	let lastSave: string | null

	$: lastSave = localStorage.getItem(path ?? 'last_save')

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

	function runTest() {
		testJobLoader.runPreview(path, code, lang, args)
	}

	async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}

	async function inferSchema() {
		let isDefault: string[] = []
		Object.entries(args).forEach(([k, v]) => {
			if (schema.properties[k].default == v) {
				isDefault.push(k)
			}
		})
		await inferArgs(lang, code, schema)
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

	onMount(() => {
		inferSchema()
	})
</script>

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window on:keydown={onKeyDown} />

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
						inferSchema()
					}}
				>
					<Editor
						bind:code
						bind:websocketAlive
						bind:this={editor}
						cmdEnterAction={() => {
							runTest()
						}}
						formatAction={async () => {
							await inferSchema()
							localStorage.setItem(path ?? 'last_save', code)
							lastSave = code
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
				<VSplitPane topPanelSize="30%" downPanelSize="70%">
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
						<div class="h-full overflow-hidden">
							<div class="px-2 py-1">
								{#if testIsLoading}
									<Button
										on:click={testJobLoader?.cancelJob}
										btnClasses="w-full"
										color="red"
										size="xs"
										startIcon={{
											icon: faRotateRight,
											classes: 'animate-spin'
										}}
									>
										'Cancel'
									</Button>
								{:else}
									<Button
										on:click={runTest}
										btnClasses="w-full"
										size="xs"
										startIcon={{
											icon: faPlay,
											classes: 'animate-none'
										}}
									>
										{testIsLoading ? 'Running' : 'Test (Ctrl+Enter)'}
									</Button>
								{/if}
							</div>
							<LogPanel
								{path}
								{lang}
								previewJob={testJob}
								{pastPreviews}
								previewIsLoading={testIsLoading}
								bind:lastSave
							/>
						</div>
					</down>
				</VSplitPane>
			</div>
		</right>
	</HSplitPane>
</div>
