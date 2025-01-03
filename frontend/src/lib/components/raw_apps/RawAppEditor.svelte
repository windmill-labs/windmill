<script lang="ts">
	import { versionRangeToVersion } from '$lib/ata'
	// import { animateTo } from '$lib/components/apps/editor/appUtils'

	import Editor from '$lib/components/Editor.svelte'
	import { extToLang } from '$lib/editorUtils'
	// import {
	// 	loadSandpackClient,
	// 	// type BundlerState,
	// 	type SandpackClient
	// } from '@codesandbox/sandpack-client'
	import { SandpackRuntime } from '@codesandbox/sandpack-client/clients/runtime'

	import { onDestroy, onMount } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { HiddenRunnable, JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy, type Preview } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { capitalize, encodeState } from '$lib/utils'

	import { schemaToTsType } from '$lib/schema'
	import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import FileEditorIcon from './FileEditorIcon.svelte'
	import { FilePlus, Pencil, TrashIcon } from 'lucide-svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { workspaceStore } from '$lib/stores'

	export let files: Record<string, { code: string }>
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

	let editor: Editor | undefined = undefined

	let runnables = writable(initRunnables)

	const WMILL_TS = '/wmill.ts'

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

	let activeFile = defaultActiveFile(files)
	console.log('activeFile', activeFile)

	function defaultActiveFile(files: Record<string, any>) {
		let fileKeys = Object.keys(files)
		const dflt = ['/App.svelte', '/App.vue', '/App.tsx']
		return fileKeys.find((x) => dflt.includes(x)) ?? fileKeys[0]
	}

	let jobs: string[] = []
	let jobsById: Record<string, JobById> = {}

	let client: SandpackRuntime | undefined = undefined
	// let bundlerState: BundlerState | undefined = undefined
	onMount(async () => {
		setSandpack()
	})

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
export function waitJob(id: string): Promise<Job>

/**
 * Get a job by id and return immediately with the current state of the job
 * @param id
 */
export function getJob(id: string): Promise<Job>
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

	function setSandpack() {
		if (iframe) {
			client = new SandpackRuntime(
				iframe,
				{
					files: addWmillClient(files),
					entry: '/index.tsx',
					dependencies: {
						react: '18.3.1',
						'react-dom': '18.3.1'
					}
				},
				{
					// bundlerURL: 'https://pub-f18889636e7746afadfd4e1bcc73fac5.r2.dev/index.html',
					// bundlerURL: '/sandpack/index.html',
					// bundlerURL: 'http://localhost:3002/sandpack/index.html',

					bundlerURL: window.origin.toString() + '/sandpack/index.html',
					showOpenInCodeSandbox: false
					// customNpmRegistries: [
					// 	{
					// 		limitToScopes: false,
					// 		registryUrl: 'http://localhost:4873/',
					// 		enabledScopes: ['*'],
					// 		proxyEnabled: false
					// 	}
					// ]
				}
			)
			client.listen((msg) => {
				console.log('Message from sandbox', msg)
			})
		}
	}

	onDestroy(() => {
		client?.destroy()
	})

	function getLangOfExt(path: string) {
		if (path.endsWith('.tsx')) return 'typescript'
		if (path.endsWith('.ts')) return 'typescript'
		if (path.endsWith('.js')) return 'javascript'
		if (path.endsWith('.jsx')) return 'typescript'
		if (path.endsWith('.css')) return 'css'
		if (path.endsWith('.json')) return 'json'
		return 'text'
	}

	function onPackageJsonChange() {
		let pkg = JSON.parse(files['/package.json'].code)
		let dependencies: Record<string, string> =
			typeof pkg.dependencies == 'object' ? pkg?.dependencies : {}
		client?.updateSandbox({
			files: addWmillClient(files),
			dependencies
		})
		const ataDeps = Object.entries(dependencies).map(([name, version]) => ({
			raw: name,
			module: name,
			version: versionRangeToVersion(version)
		}))
		editor?.fetchPackageDeps(ataDeps)
	}

	function onContentChange() {
		console.log('content change', activeFile)
		if (activeFile == '/package.json') {
			onPackageJsonChange()
			return
		}
		updateSandbox()
	}

	function updateSandbox() {
		console.log('updating sandbox')

		client?.updateSandbox({
			files: addWmillClient(files)
		})
	}
	function onActiveFileChange() {
		console.log('active file change', activeFile)
		if (activeFile != WMILL_TS) {
			editor?.switchToFile(
				activeFile,
				files[activeFile].code,
				extToLang(activeFile.split('.').pop()!)
			)
		}
	}

	let timeout: NodeJS.Timeout | undefined = undefined
	let name = ''
	let creatingNewFile = false

	$: activeFile && onActiveFileChange()

	function createNewFile(newFileName: string) {
		newFileName = '/' + newFileName
		files = { ...files, [newFileName]: { code: '' } }
		activeFile = newFileName
		creatingNewFile = false
	}

	function editFile(newFileName: string) {
		newFileName = '/' + newFileName
		files[newFileName] = files[activeFile]
		delete files[activeFile]
		activeFile = newFileName
		editingFile = undefined
		updateSandbox()
	}

	let appPanelSize = 70

	function hideBottomPanel(animate: boolean = false) {
		appPanelSize = 100
	}

	let selectedRunnable: string | undefined = undefined
	let deletingFile: string | undefined = undefined
	let editingFile: string | undefined

	function extToScriptLang(ext: string) {
		if (ext == 'ts' || ext == 'tsx') return 'tsx'
		if (ext == 'js' || ext == 'jsx') return 'jsx'
		return ext as Preview['language'] | 'json'
	}
</script>

<ConfirmationModal
	open={deletingFile != undefined}
	on:confirmed={() => {
		if (deletingFile) {
			delete files[deletingFile]
			files = files
			activeFile = Object.keys(files)[0]
			updateSandbox()
		}
		deletingFile = undefined
	}}
	on:canceled={() => (deletingFile = undefined)}
	title="Delete File {deletingFile}"
	confirmationText="Delete"
>
	Once deleted, the file will not be recoverable
</ConfirmationModal>

<RawAppBackgroundRunner
	workspace={$workspaceStore ?? ''}
	editor
	{iframe}
	bind:jobs
	bind:jobsById
	runnables={$runnables}
	{path}
/>
<div class="h-full flex flex-col">
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
	<Splitpanes id="o2" horizontal class="!overflow-visible grow">
		<Pane bind:size={appPanelSize}>
			<div class="flex h-full overflow-hidden">
				<div class="flex text-xs max-w-36 min-w-32 flex-col text-secondary">
					{#if creatingNewFile}
						<!-- svelte-ignore a11y-autofocus -->
						<input
							autofocus
							type="text"
							class="py-0"
							placeholder="file name"
							bind:value={name}
							on:keypress={(e) => {
								if (e.key == 'Enter') {
									createNewFile(name)
								} else if (e.key == 'Escape') {
									creatingNewFile = false
								}
							}}
							on:blur={() => {
								createNewFile(name)
							}}
						/>
					{:else}
						<div class="flex">
							<button
								title="New File"
								class="hover:text-primary text-left grow hover:underline hover:bg-surface-hover pl-2 pr-1 py-1"
								on:click={() => {
									name = ''

									creatingNewFile = true
								}}><FilePlus size={16} /></button
							>
							<button
								disabled={activeFile == undefined || activeFile == WMILL_TS}
								title="Rename"
								class="hover:text-primary rounded text-left hover:underline hover:bg-surface-hover py-1 px-1"
								on:click={() => {
									name = activeFile.substring(1)
									editingFile = activeFile
								}}><Pencil size={16} /></button
							>
							<button
								disabled={activeFile == undefined || activeFile == WMILL_TS}
								title="Delete"
								class="hover:text-primary rounded text-left hover:underline hover:bg-surface-hover py-1 px-1"
								on:click={() => {
									deletingFile = activeFile
								}}><TrashIcon size={16} /></button
							>
						</div>
					{/if}
					{#each Object.keys(files).concat(WMILL_TS).sort() as file}
						{#if editingFile == file}
							<!-- svelte-ignore a11y-autofocus -->
							<input
								autofocus
								type="text"
								class="py-0"
								placeholder="file name"
								bind:value={name}
								on:keypress={(e) => {
									if (e.key == 'Enter') {
										editFile(name)
									} else if (e.key == 'Escape') {
										editingFile = undefined
									}
								}}
								on:blur={() => {
									if (editingFile == '') {
										editingFile = undefined
									} else {
										editFile(name)
									}
								}}
							/>
						{:else}
							<button
								title={file.substring(1)}
								class="hover:text-primary inline-flex gap-1 text-left truncate {activeFile == file
									? 'font-bold text-primary bg-surface-selected'
									: ''} hover:underline hover:bg-surface-hover rounded pl-2 pr-1 py-1"
								on:click={() => {
									activeFile = file
								}}
							>
								<span class="w-4 min-w-4"><FileEditorIcon {file} /></span>
								<span class="truncate">{file.substring(1)}</span></button
							>
						{/if}
					{/each}
				</div>
				<div class="w-full grid grid-cols-2">
					{#if activeFile == WMILL_TS}
						<Editor
							lang={getLangOfExt(activeFile)}
							path={activeFile}
							scriptLang="tsx"
							bind:this={editor}
							code={genWmillTs($runnables)}
							{files}
						/>
					{:else}
						<Editor
							lang={getLangOfExt(activeFile)}
							path={activeFile}
							scriptLang={extToScriptLang(activeFile.split('.').pop() ?? 'tsx')}
							bind:this={editor}
							bind:code={files[activeFile].code}
							files={addWmillClientDts(files, $runnables)}
							on:ataReady={() => {
								onPackageJsonChange()
							}}
							on:change={() => {
								console.log('bar')
								timeout && clearTimeout(timeout)
								timeout = setTimeout(() => {
									onContentChange()
								}, 500)
							}}
						/>
					{/if}
					<div class="h-full w-full relative">
						<button
							class="absolute top-0 right-0 z-50"
							on:click={() => {
								// client?.updateOptions({})
								client?.destroy()
								setSandpack()
							}}>reload</button
						>
						<!-- svelte-ignore a11y-missing-attribute -->
						<iframe class="min-h-screen w-full bg-white" bind:this={iframe} />
					</div>
				</div>
			</div>
		</Pane>
		<Pane>
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div class="flex h-full w-full overflow-x-visible">
				<RawAppInlineScriptsPanel
					appPath={path}
					bind:selectedRunnable
					{runnables}
					on:hidePanel={() => hideBottomPanel(true)}
					on:hidePanel
				/>
			</div>
			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
</div>

<style>
</style>
