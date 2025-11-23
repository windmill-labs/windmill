<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { encodeState } from '$lib/utils'

	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { genWmillTs, type Runnable } from './utils'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import RawAppSidebar from './RawAppSidebar.svelte'
	import type { Modules } from './RawAppModules.svelte'

	interface Props {
		initFiles: Record<string, string>
		initRunnables: Record<string, Runnable>
		newApp: boolean
		policy: Policy
		summary?: string
		path: string
		newPath?: string | undefined
		savedApp?:
			| {
					value: any
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					custom_path?: string
			  }
			| undefined
		diffDrawer?: DiffDrawer | undefined
	}

	let {
		initFiles,
		initRunnables,
		newApp,
		policy,
		summary = $bindable(''),
		path,
		newPath = undefined,
		savedApp = $bindable(undefined),
		diffDrawer = undefined
	}: Props = $props()
	export const version: number | undefined = undefined

	let runnables = $state(initRunnables)

	let initRunnablesContent = Object.fromEntries(
		Object.entries(initRunnables).map(([key, runnable]) => {
			if (runnable?.type === 'runnableByName') {
				return [key, runnable?.inlineScript?.content ?? '']
			}
			return [key, '']
		})
	)

	let files: Record<string, string> | undefined = $state(initFiles)

	let draftTimeout: number | undefined = undefined
	function saveFrontendDraft() {
		draftTimeout && clearTimeout(draftTimeout)
		draftTimeout = setTimeout(() => {
			try {
				localStorage.setItem(
					path != '' ? `rawapp-${path}` : 'rawapp',
					encodeState({
						files,
						runnables: runnables
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	let iframe: HTMLIFrameElement | undefined = $state(undefined)

	let sidebarPanelSize = $state(10)

	let jobs: string[] = $state([])
	let jobsById: Record<string, JobById> = $state({})

	let iframeLoaded = $state(false) // @hmr:keep

	function populateFiles() {
		setFilesInIframe(initFiles)
	}
	function setFilesInIframe(newFiles: Record<string, string>) {
		const files = Object.fromEntries(
			Object.entries(newFiles).filter(([path, _]) => !path.endsWith('/'))
		)
		iframe?.contentWindow?.postMessage(
			{
				type: 'setFiles',
				files: files
			},
			'*'
		)
	}

	function populateRunnables() {
		iframe?.contentWindow?.postMessage(
			{
				type: 'setRunnables',
				dts: genWmillTs(runnables)
			},
			'*'
		)
	}

	let selectedRunnable: string | undefined = $state(undefined)
	let selectedDocument: string | undefined = $state(undefined)

	let modules = $state({}) as Modules
	function listener(e: MessageEvent) {
		if (e.data.type === 'setFiles') {
			files = e.data.files
		} else if (e.data.type === 'getBundle') {
			getBundleResolve?.(e.data.bundle)
		} else if (e.data.type === 'updateModules') {
			modules = e.data.modules
		} else if (e.data.type === 'setActiveDocument') {
			selectedDocument = e.data.path
		}
	}

	let getBundleResolve: (({ css, js }: { css: string; js: string }) => void) | undefined = undefined

	async function getBundle(): Promise<{ css: string; js: string }> {
		return new Promise((resolve) => {
			getBundleResolve = resolve
			iframe?.contentWindow?.postMessage(
				{
					type: 'getBundle'
				},
				'*'
			)
		})
	}

	let darkMode: boolean = $state(false)
	$effect(() => {
		runnables && files && saveFrontendDraft()
	})
	$effect(() => {
		iframe?.addEventListener('load', () => {
			iframeLoaded = true
		})
	})
	$effect(() => {
		iframe && iframeLoaded && initFiles && populateFiles()
	})
	$effect(() => {
		iframe && iframeLoaded && runnables && populateRunnables()
	})

	function handleSelectFile(path: string) {
		console.log('event Select file:', path)
		selectedRunnable = undefined
		iframe?.contentWindow?.postMessage(
			{
				type: 'selectFile',
				path: path
			},
			'*'
		)
	}
</script>

<svelte:window onmessage={listener} />
<DarkModeObserver bind:darkMode />

<RawAppBackgroundRunner
	workspace={$workspaceStore ?? ''}
	editor
	{iframe}
	bind:jobs
	bind:jobsById
	{runnables}
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
		{getBundle}
	/>

	<Splitpanes id="o2" class="grow">
		<Pane bind:size={sidebarPanelSize} maxSize={20}>
			<RawAppSidebar
				bind:files={
					() => files,
					(newFiles) => {
						files = newFiles
						setFilesInIframe(newFiles ?? {})
					}
				}
				onSelectFile={handleSelectFile}
				bind:selectedRunnable
				bind:selectedDocument
				{runnables}
				{modules}
			></RawAppSidebar>
		</Pane>
		<Pane>
			<iframe
				bind:this={iframe}
				title="UI builder"
				style="display: {selectedRunnable == undefined ? 'block' : 'none'}"
				src="/ui_builder/index.html?dark={darkMode}"
				class="w-full h-full"
			></iframe>
			{#if selectedRunnable !== undefined}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="flex h-full w-full">
					<RawAppInlineScriptsPanel
						appPath={path}
						{selectedRunnable}
						{initRunnablesContent}
						{runnables}
					/>
				</div>
			{/if}

			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
</div>
