<script lang="ts">
	import { run } from 'svelte/legacy'

	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	import RawAppInlineScriptsPanel from './RawAppInlineScriptsPanel.svelte'
	import type { HiddenRunnable, JobById } from '../apps/types'
	import RawAppEditorHeader from './RawAppEditorHeader.svelte'
	import { type Policy } from '$lib/gen'
	import DiffDrawer from '../DiffDrawer.svelte'
	import { encodeState } from '$lib/utils'

	// import { addWmillClient } from './utils'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { workspaceStore } from '$lib/stores'
	import { genWmillTs } from './utils'
	import HideButton from '../apps/editor/settingsPanel/HideButton.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'

	interface Props {
		initFiles: Record<string, string>
		initRunnables: Record<string, HiddenRunnable>
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

	let runnables = writable(initRunnables)

	let files: Record<string, string> | undefined = $state(initFiles)

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

	let iframe: HTMLIFrameElement | undefined = $state(undefined)

	let appPanelSize = $state(70)

	let jobs: string[] = $state([])
	let jobsById: Record<string, JobById> = $state({})

	let iframeLoaded = $state(false) // @hmr:keep

	function populateFiles() {
		iframe?.contentWindow?.postMessage(
			{
				type: 'setFiles',
				files: initFiles
			},
			'*'
		)
	}

	function populateRunnables() {
		iframe?.contentWindow?.postMessage(
			{
				type: 'setRunnables',
				dts: genWmillTs($runnables)
			},
			'*'
		)
	}

	let selectedRunnable: string | undefined = $state(undefined)

	function listener(e: MessageEvent) {
		if (e.data.type === 'setFiles') {
			files = e.data.files
		} else if (e.data.type === 'getBundle') {
			getBundleResolve?.(e.data.bundle)
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
	run(() => {
		$runnables && files && saveFrontendDraft()
	})
	run(() => {
		iframe?.addEventListener('load', () => {
			iframeLoaded = true
		})
	})
	run(() => {
		iframe && iframeLoaded && initFiles && populateFiles()
	})
	run(() => {
		iframe && iframeLoaded && $runnables && populateRunnables()
	})
</script>

<svelte:window onmessage={listener} />
<DarkModeObserver bind:darkMode />

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
		{getBundle}
	/>
	<Splitpanes id="o2" horizontal class="grow">
		<Pane bind:size={appPanelSize}>
			<!-- <iframe
				bind:this={iframe}
				title="UI builder"
				src="http://localhost:4000/ui_builder/index.html?dark={darkMode}"
				class="w-full h-full"
			></iframe> -->
			<iframe
				bind:this={iframe}
				title="UI builder"
				src="/ui_builder/index.html?dark={darkMode}"
				class="w-full h-full"
			></iframe>
		</Pane>
		<Pane>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="flex h-full w-full">
				<RawAppInlineScriptsPanel
					on:hidePanel={() => {
						appPanelSize = 100
					}}
					appPath={path}
					bind:selectedRunnable
					{runnables}
				/>
			</div>
			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
	{#if appPanelSize == 100}
		<div class="absolute bottom-0.5 left-0.5 z-50">
			<HideButton
				size="lg"
				on:click={() => {
					appPanelSize = 70
				}}
				direction="bottom"
				hidden
				btnClasses="border bg-surface"
			/>
		</div>
	{/if}
</div>
