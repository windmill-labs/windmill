<script lang="ts">
	import type { Schema } from '$lib/common'
	import { CompletedJob, Job, JobService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		emptySchema,
		getModifierKey,
		isCloudHosted,
		scriptLangToEditorLang,
		sendUserToast
	} from '$lib/utils'
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
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { Button, Kbd } from './common'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import * as Y from 'yjs'
	import { WebsocketProvider } from 'y-websocket'

	// Exported
	export let schema: Schema = emptySchema()
	export let code: string
	export let path: string | undefined
	export let lang: Preview.language
	export let kind: string | undefined = undefined
	export let tag: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let fixedOverflowWidgets = true
	export let noSyncFromGithub = false
	export let editor: Editor | undefined = undefined
	export let collabMode = false

	let websocketAlive = { pyright: false, black: false, deno: false, go: false }

	let width = 1200

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
		testJobLoader.runPreview(path, code, lang, args, tag)
	}

	async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}

	export async function inferSchema(code: string, nlang?: 'go' | 'bash' | 'python3' | 'deno') {
		schema = schema ?? emptySchema()
		let isDefault: string[] = []
		Object.entries(args).forEach(([k, v]) => {
			if (schema.properties?.[k]?.default == v) {
				isDefault.push(k)
			}
		})

		try {
			await inferArgs(nlang ?? lang, code, schema)
			validCode = true
		} catch (e) {
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
		loadPastTests()
	})

	let wsProvider: WebsocketProvider | undefined = undefined
	let yContent: Y.Text | undefined = undefined
	let yMeta: Y.Map<any> | undefined = undefined

	$: collabLive && lang && yMeta && yMeta?.set('lang', lang)

	let collabLive = false
	export function setCollaborationMode() {
		sendUserToast(
			`Live sharing enabled. ${
				isCloudHosted()
					? ''
					: 'Premium feature available during beta and only on premium plans afterwards.'
			}`
		)
		console.log('collab mode')
		collabLive = true
		const ydoc = new Y.Doc({})
		yContent = ydoc.getText('content')
		wsProvider = new WebsocketProvider('ws://localhost:1234', 'my-roomname', ydoc)

		wsProvider.on('sync', (isSynced) => {
			if (yContent?.toJSON() == '') {
				yContent?.insert(0, code)
			}
			if (yMeta?.get('lang') == undefined) {
				yMeta?.set('lang', lang)
			}
		})

		yMeta = ydoc.getMap('meta')

		yMeta.observeDeep(() => {
			if (yMeta?.get('lang') != undefined) {
				lang = yMeta?.get('lang')
			}
		})

		// All of our network providers implement the awareness crdt
		const awareness = wsProvider.awareness
		// You can observe when a user updates their awareness information
		awareness.on('change', (changes) => {
			// Whenever somebody updates their awareness information,
			// we log all awareness information from all users.
			console.log(Array.from(awareness.getStates().values()))
		})

		// You can think of your own awareness information as a key-value store.
		// We update our "user" field to propagate relevant user information.
		awareness.setLocalStateField('user', {
			// Define a print name that should be displayed
			name: $userStore?.username,
			// Define a color that should be associated to the user:
			color: '#999999' // should be a hex color
		})
	}

	export function disableCollaboration() {
		console.log('collab mode disabled')
		collabLive = false
		wsProvider?.disconnect()
	}

	onDestroy(() => {
		wsProvider?.disconnect()
	})

	const dispatch = createEventDispatcher()

	function asKind(str: string | undefined) {
		return str as 'script' | 'approval' | 'trigger' | undefined
	}
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
			on:toggleCollabMode={() => {
				if (collabLive) {
					disableCollaboration()
				} else {
					setCollaborationMode()
				}
			}}
			bind:collabLive
			{collabMode}
			{validCode}
			iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
			{editor}
			{lang}
			{websocketAlive}
			kind={asKind(kind)}
		/>
		{#if !noSyncFromGithub}
			<div class="py-1">
				<Button
					target="_blank"
					href="https://github.com/windmill-labs/windmill/tree/main/cli"
					color="light"
					size="xs"
					btnClasses="mr-1 hidden lg:block"
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
			<div class="pl-2 h-full !overflow-visible">
				{#key lang}
					<Editor
						{path}
						bind:code
						bind:websocketAlive
						bind:this={editor}
						{yContent}
						awareness={wsProvider?.awareness}
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
							dispatch('format')
						}}
						class="flex flex-1 h-full !overflow-visible"
						lang={scriptLangToEditorLang(lang)}
						automaticLayout={true}
						{fixedOverflowWidgets}
					/>
				{/key}
			</div>
		</Pane>
		<Pane size={40} minSize={10}>
			<div class="flex flex-col h-full">
				<div class="flex justify-center pt-1">
					{#if testIsLoading}
						<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
							<WindmillIcon
								white={true}
								class="mr-2 text-white"
								height="20px"
								width="20px"
								spin="fast"
							/>
							Cancel
						</Button>
					{:else}
						<Button
							color="dark"
							on:click={() => {
								runTest()
							}}
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
								Test&nbsp;<Kbd small>{getModifierKey()}</Kbd>
								<Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
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
