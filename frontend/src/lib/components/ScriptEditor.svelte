<script lang="ts">
	import { BROWSER } from 'esm-env'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { CompletedJob, Job, JobService, SettingsService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { copyToClipboard, emptySchema, getModifierKey, sendUserToast } from '$lib/utils'
	import Editor from './Editor.svelte'
	import { inferArgs } from '$lib/infer'
	import type { Preview } from '$lib/gen/models/Preview'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SchemaForm from './SchemaForm.svelte'
	import LogPanel from './scriptEditor/LogPanel.svelte'
	import EditorBar, { EDITOR_BAR_WIDTH_THRESHOLD } from './EditorBar.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { Button, Kbd } from './common'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import * as Y from 'yjs'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { WebsocketProvider } from 'y-websocket'
	import Modal from './common/modal/Modal.svelte'
	import DiffEditor from './DiffEditor.svelte'
	import { Clipboard, Github, Play } from 'lucide-svelte'

	// Exported
	export let schema: Schema | any = emptySchema()
	export let code: string
	export let path: string | undefined
	export let lang: Preview.language
	export let kind: string | undefined = undefined
	export let template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' = 'script'
	export let tag: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let fixedOverflowWidgets = true
	export let noSyncFromGithub = false
	export let editor: Editor | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined
	export let collabMode = false
	export let edit = true

	let websocketAlive = {
		pyright: false,
		black: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false,
		bun: false
	}

	let width = 1200

	let testJobLoader: TestJobLoader

	// Test args input
	let args: Record<string, any> = initialArgs

	let isValid: boolean = true

	// Test
	let testIsLoading = false
	let testJob: Job | undefined
	let pastPreviews: CompletedJob[] = []
	let validCode = true

	let wsProvider: WebsocketProvider | undefined = undefined
	let yContent: Y.Text | undefined = undefined
	let peers: { name: string }[] = []
	let showCollabPopup = false

	const url = new URL(window.location.toString())
	let initialCollab = /true|1/i.test(url.searchParams.get('collab') ?? '0')

	if (initialCollab) {
		setCollaborationMode()
		url.searchParams.delete('collab')
		url.searchParams.delete('path')
		history.replaceState(null, '', url)
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

	function runTest() {
		//@ts-ignore
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

	export async function inferSchema(code: string, nlang?: SupportedLanguage) {
		let nschema = schema ?? emptySchema()

		try {
			await inferArgs(nlang ?? lang, code, nschema)
			validCode = true
			schema = nschema
		} catch (e) {
			validCode = false
		}
	}

	onMount(() => {
		inferSchema(code)
		loadPastTests()
	})

	export async function setCollaborationMode() {
		if (!$enterpriseLicense) {
			$enterpriseLicense = await SettingsService.getLicenseId()
		}

		if (!$enterpriseLicense) {
			sendUserToast(`Multiplayer is an enterprise feature`, true, [
				{
					label: 'Upgrade',
					callback: () => {
						window.open('https://www.windmill.dev/pricing', '_blank')
					}
				}
			])
			return
		}

		const ydoc = new Y.Doc()
		if (wsProvider) {
			wsProvider.destroy()
		}
		let yContentInit = ydoc.getText('content')

		const wsProtocol = BROWSER && window.location.protocol == 'https:' ? 'wss' : 'ws'

		wsProvider = new WebsocketProvider(
			`${wsProtocol}://${window.location.host}/ws_mp/`,
			$workspaceStore + '/' + path ?? 'no-room-name',
			ydoc,
			{ connect: false }
		)

		wsProvider.on('sync', (isSynced: boolean) => {
			if (isSynced && yContentInit?.toJSON() == '') {
				showCollabPopup = true
				yContentInit?.insert(0, code)
			}
			yContent = yContentInit
		})

		wsProvider.on('connection-error', (WSErrorEvent) => {
			console.error(WSErrorEvent)
			sendUserToast('Multiplayer server connection had an error', true)
		})
		wsProvider.connect()
		const awareness = wsProvider.awareness

		awareness.setLocalStateField('user', {
			name: $userStore?.username
		})

		function setPeers() {
			peers = Array.from(awareness.getStates().values()).map((x) => x.user)
		}

		setPeers()
		// You can observe when a user updates their awareness information
		awareness.on('change', (changes) => {
			setPeers()
		})
	}

	export function disableCollaboration() {
		if (!wsProvider?.shouldConnect) return
		peers = []
		console.log('collab mode disabled')
		wsProvider?.disconnect()
		wsProvider.destroy()
		wsProvider = undefined
	}

	onDestroy(() => {
		disableCollaboration()
	})

	const dispatch = createEventDispatcher()

	function asKind(str: string | undefined) {
		return str as 'script' | 'approval' | 'trigger' | undefined
	}

	function collabUrl() {
		let url = new URL(window.location.toString())
		url.search = ''
		return `${url}?collab=1` + (edit ? '' : `&path=${path}`)
	}
</script>

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window on:keydown={onKeyDown} />

<Modal title="Invite others" bind:open={showCollabPopup}>
	<div>Have others join by sharing the following url:</div>
	<div class="flex gap-2 pr-4">
		<input type="text" disabled value={collabUrl()} />

		<Button
			color="light"
			startIcon={{ icon: Clipboard }}
			iconOnly
			on:click={() => copyToClipboard(collabUrl())}
		/>
	</div>
</Modal>
<div class="border-b-2 shadow-sm px-1 pr-4" bind:clientWidth={width}>
	<div class="flex justify-between space-x-2">
		<EditorBar
			scriptPath={edit ? path : undefined}
			on:toggleCollabMode={() => {
				if (wsProvider?.shouldConnect) {
					disableCollaboration()
				} else {
					setCollaborationMode()
				}
			}}
			collabLive={wsProvider?.shouldConnect}
			{collabMode}
			{validCode}
			iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
			on:collabPopup={() => (showCollabPopup = true)}
			{editor}
			{lang}
			{websocketAlive}
			collabUsers={peers}
			kind={asKind(kind)}
			{template}
			{diffEditor}
			{args}
		/>
		{#if !noSyncFromGithub}
			<div class="py-1">
				<Button
					target="_blank"
					href="https://www.windmill.dev/docs/cli_local_dev/vscode-extension"
					color="light"
					size="xs"
					btnClasses="hidden lg:flex"
					startIcon={{
						icon: Github
					}}
				>
					Use VScode
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
						folding
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
							dispatch('format')
						}}
						class="flex flex-1 h-full !overflow-visible"
						lang={scriptLangToEditorLang(lang)}
						deno={lang == 'deno'}
						automaticLayout={true}
						{fixedOverflowWidgets}
						{args}
					/>
					<DiffEditor
						bind:this={diffEditor}
						automaticLayout
						{fixedOverflowWidgets}
						class="hidden h-full"
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
								height="16px"
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
								icon: Play,
								classes: 'animate-none'
							}}
						>
							{#if testIsLoading}
								Running
							{:else}
								Test&nbsp;<Kbd small isModifier>{getModifierKey()}</Kbd>
								<Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
							{/if}
						</Button>
					{/if}
				</div>
				<Splitpanes horizontal class="!max-h-[calc(100%-43px)]">
					<Pane size={33}>
						<div class="px-2">
							<div class="break-words relative font-sans">
								<SchemaForm compact {schema} bind:args bind:isValid showSchemaExplorer />
							</div>
						</div>
					</Pane>
					<Pane size={67}>
						<LogPanel
							{lang}
							previewJob={testJob}
							{pastPreviews}
							previewIsLoading={testIsLoading}
							{editor}
							{diffEditor}
							{args}
						/>
					</Pane>
				</Splitpanes>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
