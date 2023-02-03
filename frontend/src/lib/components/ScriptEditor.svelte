<script lang="ts">
	import type { Schema } from '$lib/common'
	import { CompletedJob, Job, JobService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, scriptLangToEditorLang } from '$lib/utils'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'
	import Editor from './Editor.svelte'
	import { inferArgs } from '$lib/infer'
	import type { Preview } from '$lib/gen/models/Preview'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SchemaForm from './SchemaForm.svelte'
	import LogPanel from './scriptEditor/LogPanel.svelte'
	import { faGithub } from '@fortawesome/free-brands-svg-icons'
	import EditorBar, { EDITOR_BAR_WIDTH_THRESHOLD } from './EditorBar.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import { onMount } from 'svelte'
	import { Button, Kbd } from './common'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'

	// Exported
	export let schema: Schema = emptySchema()
	export let code: string
	export let path: string | undefined
	export let lang: Preview.language
	export let kind: 'script' | 'trigger' | 'approval' | undefined = undefined
	export let initialArgs: Record<string, any> = {}
	export let fixedOverflowWidgets = true
	export let noSyncFromGithub = false

	let websocketAlive = { pyright: false, black: false, deno: false, go: false }

	let width = 1200

	// Internal state
	let editor: Editor

	let testJobLoader: TestJobLoader

	// Test args input
	let args: Record<string, any> = initialArgs
	let isValid: boolean = true

	// Test
	let testIsLoading = false
	let testJob: Job | undefined
	let pastPreviews: CompletedJob[] = []
	let lastSave: string | null
	let validCode = true

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

	async function inferSchema(code: string) {
		schema = schema ?? emptySchema()
		let isDefault: string[] = []
		Object.entries(args).forEach(([k, v]) => {
			if (schema.properties[k].default == v) {
				isDefault.push(k)
			}
		})

		try {
			await inferArgs(lang, code, schema)
			validCode = true
		} catch (e) {
			console.error("Couldn't infer args", e)
			validCode = false
		}

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
		inferSchema(code)
	})
</script>

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window on:keydown={onKeyDown} />

<div class="border-b-2 shadow-sm px-1 pr-4" bind:clientWidth={width}>
	<div class="flex justify-between space-x-2">
		<EditorBar
			{validCode}
			iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
			{editor}
			{lang}
			{websocketAlive}
			{kind}
		/>
		{#if !noSyncFromGithub}
			<div class="py-1">
				<Button
					target="_blank"
					href="https://github.com/windmill-labs/windmill/tree/main/cli"
					color="light"
					size="xs"
					btnClasses="mr-1 hidden md:block"
					startIcon={{
						icon: faGithub
					}}
				>
					Sync from Github
				</Button>
			</div>
		{/if}
	</div>
</div>
<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane size={60} minSize={10} class="!overflow-visible">
			<div
				class="pl-2 h-full !overflow-visible"
				on:mouseleave={() => {
					inferSchema(code)
				}}
			>
				<Editor
					bind:code
					bind:websocketAlive
					bind:this={editor}
					on:change={(e) => {
						inferSchema(e.detail)
					}}
					cmdEnterAction={async () => {
						await inferSchema(code)
						runTest()
					}}
					formatAction={async () => {
						await inferSchema(code)
						try {
							localStorage.setItem(path ?? 'last_save', code)
						} catch (e) {
							console.error('Could not save last_save to local storage', e)
						}
						lastSave = code
					}}
					class="flex flex-1 h-full !overflow-visible"
					lang={scriptLangToEditorLang(lang)}
					automaticLayout={true}
					{fixedOverflowWidgets}
				/>
			</div>
		</Pane>
		<Pane size={40} minSize={10}>
			<div class="flex flex-col h-full">
				<div class="px-2 w-full border-b py-1">
					{#if testIsLoading}
						<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
							<WindmillIcon
								white={true}
								class="animate-[spin_5s_linear_infinite] mr-2 text-white"
								height="20px"
								width="20px"
							/>
							Cancel
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
							{#if testIsLoading}
								Running
							{:else}
								Test <Kbd class="ml-4 text-5xs -my-0.5">Ctrl+Enter</Kbd>
							{/if}
						</Button>
					{/if}
				</div>
				<Splitpanes horizontal class="!max-h-[calc(100%-43px)]">
					<Pane size={33}>
						<div class="px-2">
							<div class="break-words relative font-sans">
								<SchemaForm compact {schema} bind:args bind:isValid />
							</div>
						</div>
					</Pane>
					<Pane size={67}>
						<LogPanel
							{path}
							{lang}
							previewJob={testJob}
							{pastPreviews}
							previewIsLoading={testIsLoading}
							bind:lastSave
						/>
					</Pane>
				</Splitpanes>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
