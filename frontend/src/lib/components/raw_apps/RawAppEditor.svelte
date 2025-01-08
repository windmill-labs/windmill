<script lang="ts">
	import { versionRangeToVersion } from '$lib/ata'
	// import { animateTo } from '$lib/components/apps/editor/appUtils'

	// import Editor from '$lib/components/Editor.svelte'
	// import { extToLang } from '$lib/editorUtils'
	// import {
	// 	loadSandpackClient,
	// 	// type BundlerState,
	// 	type SandpackClient
	// } from '@codesandbox/sandpack-client'
	// import { SandpackRuntime } from '@codesandbox/sandpack-client/clients/runtime'

	import { onDestroy } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { HiddenRunnable, JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy, type Preview } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { capitalize, encodeState, sendUserToast } from '$lib/utils'

	import { schemaToTsType } from '$lib/schema'
	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import FileEditorIcon from './FileEditorIcon.svelte'
	import { ChevronDown, ChevronUp, FilePlus, Pencil, TrashIcon } from 'lucide-svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { workspaceStore } from '$lib/stores'
	import EsbuildBundler from './EsbuildBundler.svelte'
	import RawAppCodeEditor from './RawAppCodeEditor.svelte'
	import Toggle from '../Toggle.svelte'
	import { wmillTs } from './utils'
	import type { InstalledPackage } from './npm_install'

	export let files: Record<string, string>
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
	let popup: WindowProxy | null = null // @hmr:keep

	let activeFile = defaultActiveFile(files)

	let logsDiv: HTMLDivElement | undefined = undefined
	let logsCollapsed = false

	$: logs && logsDiv && scrollToBottom()

	function scrollToBottom() {
		setTimeout(() => {
			logsDiv?.scrollTo(0, logsDiv.scrollHeight)
		}, 100)
	}

	function defaultActiveFile(files: Record<string, any>) {
		let fileKeys = Object.keys(files)
		const dflt = ['/App.svelte', '/App.vue', '/App.tsx']
		return fileKeys.find((x) => dflt.includes(x)) ?? fileKeys[0]
	}

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

	onDestroy(() => {
		popup?.close()
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
		let pkg = JSON.parse(files['/package.json'])
		let dependencies: Record<string, string> =
			typeof pkg.dependencies == 'object' ? pkg?.dependencies : {}

		const ataDeps = Object.entries(dependencies).map(([name, version]) => ({
			raw: name,
			module: name,
			version: versionRangeToVersion(version)
		}))
		// editor?.fetchPackageDeps(ataDeps)
	}

	function onActiveFileChange() {
		console.log('active file change', activeFile)
		if (activeFile != WMILL_TS) {
			// editor?.switchToFile(
			// 	activeFile,
			// 	files[activeFile].code,
			// 	extToLang(activeFile.split('.').pop()!)
			// )
		}
	}

	let name = ''
	let creatingNewFile = false

	$: activeFile && onActiveFileChange()

	function createNewFile(newFileName: string) {
		newFileName = '/' + newFileName
		files = { ...files, [newFileName]: '' }
		activeFile = newFileName
		creatingNewFile = false
	}

	function editFile(newFileName: string) {
		newFileName = '/' + newFileName
		if (newFileName == editingFile) {
			editingFile = undefined
			return
		}
		files[newFileName] = files[activeFile]
		delete files[activeFile]
		activeFile = newFileName
		editingFile = undefined
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
	let bundler: EsbuildBundler | undefined = undefined
	let logs = ''

	function sendPreview(window: Window | null, css: string, js: string) {
		window?.postMessage({ type: 'preview', css: css, js: js }, '*')
	}
	function setPopupWindow(css: string, js: string) {
		if (!popup || popup?.closed) {
			popup = window.open('/app-preview.html', '_blank')
		}
		if (popup) {
			sendPreview(popup, css, js)
			popup.addEventListener('load', () => {
				sendPreview(popup, css, js)
			})
		} else {
			sendUserToast('Could not open popup window', true)
		}
	}

	let lastBuild: { js: string; css: string } | undefined = undefined
	function onBuild(js: string, css: string) {
		lastBuild = { js, css }
		console.log('onBuild')
		if (iframe) {
			let wd = iframe.contentWindow
			sendPreview(wd, css, js)
			iframe.addEventListener('load', () => {
				sendPreview(wd, css, js)
			})
		}
		if (false) {
			setPopupWindow(css, js)
		}
	}

	let visible = true
	let installed: InstalledPackage[] = []
</script>

<EsbuildBundler
	on:build={(e) => {
		onBuild(e.detail.js, e.detail.css)
	}}
	on:buildFailed={(e) => {
		sendUserToast('Build failed', true)
	}}
	on:install={(e) => {
		installed = e.detail
		// npm_install(e.detail.name, e.detail.spec)
	}}
	bind:logs
	{files}
	bind:this={bundler}
/>
<ConfirmationModal
	open={deletingFile != undefined}
	on:confirmed={() => {
		if (deletingFile) {
			delete files[deletingFile]
			files = files
			activeFile = Object.keys(files)[0]
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
			<div class="flex h-full relative">
				{#if false}
					<div class="flex text-xs max-w-36 min-w-32 flex-col text-secondary overflow-auto">
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
									class="hover:text-primary inline-flex min-h-6 gap-1 text-left truncate {activeFile ==
									file
										? 'font-bold text-primary bg-surface-selected'
										: ''} hover:underline hover:bg-surface-hover rounded pl-2 pr-1 py-1"
									on:click={() => {
										// files[activeFile] = { code: editor?.getCode() ?? '' }
										activeFile = file
									}}
								>
									<span class="w-4 min-w-4"><FileEditorIcon {file} /></span>
									<span class="truncate">{file.substring(1)}</span></button
								>
							{/if}
						{/each}
					</div>
				{/if}
				<Splitpanes class="grow">
					<Pane size={50}>
						<!-- {#if activeFile == WMILL_TS}
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
							class="min-h-0"
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
								bundler?.onContentChange(activeFile)
							}}
							changeTimeout={2000}
						/>
					{/if} -->
						<div class="absolute top-0 z-10 left-0">
							<Toggle bind:checked={visible} />
						</div>
						{#if visible}
							<RawAppCodeEditor
								user_files={files}
								node_modules={Object.fromEntries(
									installed.flatMap((e) =>
										e.files.map((f) => ['/node_modules/' + e.name + '/' + f.path, f.content])
									)
								)}
								wmill_ts={genWmillTs($runnables)}
							/>
						{:else}
							<div>i</div>
						{/if}
					</Pane>
					<Pane size={50}>
						<div class="h-full max-h-full min-h-32 w-full relative">
							<button
								class="absolute top-0 right-0"
								on:click={() => {
									lastBuild && onBuild(lastBuild.js, lastBuild.css)
								}}>reload</button
							>
							<!-- svelte-ignore a11y-missing-attribute -->
							<iframe bind:this={iframe} src="/app-preview.html" class="h-full w-full" />
						</div></Pane
					>
				</Splitpanes>
				<div
					class="p-1 text-2xs border rounded rounded-tl-md rounded-rl-none rounded-bl-none rounded-br-none text-secondary absolute right-0 bottom-0 max-w-[500px] w-full {logsCollapsed
						? 'h-6'
						: 'max-h-60 h-full'} bg-surface-secondary"
				>
					<div bind:this={logsDiv} class="h-full w-full overflow-auto">
						{#if !logsCollapsed}
							<pre>{logs}</pre>
						{/if}
					</div>
					<div>
						<button
							class="bg-surface-secondary absolute flex items-center gap-2 top-0 left-0 w-full text-md -mt-0.5 px-2 text-left"
							on:click={() => {
								logsCollapsed = !logsCollapsed
							}}
							>Logs {#if logsCollapsed}<ChevronUp />{:else}<ChevronDown />{/if}
							<div class="text-tertiary">({logs.split('\n').length})</div></button
						>
					</div>
				</div></div
			></Pane
		>
		<Pane>
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div class="flex h-full w-full">
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
