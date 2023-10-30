<script lang="ts">
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'

	import { emptySchema, sendUserToast } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { onDestroy, onMount, setContext } from 'svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import FlowPreviewButtons from '$lib/components/flows/header/FlowPreviewButtons.svelte'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { writable } from 'svelte/store'
	import type { Flow, FlowModule, Job } from '$lib/gen'
	import { initHistory } from '$lib/history'
	import type { FlowState } from '$lib/components/flows/flowState'
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import FlowEditorPanel from '$lib/components/flows/content/FlowEditorPanel.svelte'

	let testJobLoader: TestJobLoader

	// Test
	let testIsLoading = false
	let testJob: Job | undefined

	const flowStore = writable({
		path: '',
		summary: '',
		value: { modules: [] },
		edited_by: '',
		edited_at: '',
		archived: false,
		extra_perms: {},
		schema: emptySchema()
	} as Flow)
	const flowStateStore = writable({} as FlowState)
	const scheduleStore = writable({
		args: {},
		cron: '',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		enabled: false
	})
	const previewArgsStore = writable<Record<string, any>>({})
	const scriptEditorDrawer = writable(undefined)
	const moving = writable<{ module: FlowModule; modules: FlowModule[] } | undefined>(undefined)
	const history = initHistory($flowStore)

	const testStepStore = writable<Record<string, any>>({})
	const selectedIdStore = writable('')

	// function select(selectedId: string) {
	// 	selectedIdStore.set(selectedId)
	// }

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		moving,
		history,
		flowStateStore,
		flowStore,
		testStepStore,
		saveDraft: () => {},
		initialPath: ''
	})
	type LastEdit = {
		content: string
		path: string
	}

	let currentScript: LastEdit | undefined = undefined

	const href = window.location.href
	const indexQ = href.indexOf('?')
	const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined

	if (searchParams?.has('local')) {
		connectWs()
	}

	const el = (event) => {
		if (event.data.type == 'runTest') {
			runTest()
		} else if (event.data.type == 'replaceScript') {
			replaceScript(event.data)
		}
	}

	onMount(() => {
		window.addEventListener('message', el, false)
		document.addEventListener('keydown', (e) => {
			const obj = {
				altKey: e.altKey,
				code: e.code,
				ctrlKey: e.ctrlKey,
				isComposing: e.isComposing,
				key: e.key,
				location: e.location,
				metaKey: e.metaKey,
				repeat: e.repeat,
				shiftKey: e.shiftKey
			}
			window.parent?.postMessage(JSON.stringify(obj), '*')
		})
	})

	onDestroy(() => {
		window.removeEventListener('message', el)
	})

	function connectWs() {
		const port = searchParams?.get('port') || '3001'
		try {
			const socket = new WebSocket(`ws://localhost:${port}/ws`)

			// Listen for messages
			socket.addEventListener('message', (event) => {
				replaceData(event.data)
			})

			function replaceData(msg: string) {
				let data: any | undefined = undefined
				try {
					data = JSON.parse(msg)
				} catch {
					console.log('Received invalid JSON: ' + msg)
					return
				}
				replaceScript(data)
			}
		} catch (e) {
			sendUserToast('Failed to connect to local server', true)
			console.error(e)
		}
	}

	function runTest() {
		if (!currentScript) {
			return
		}
		//@ts-ignore
		testJobLoader.runPreview(currentScript.path, currentScript.content, args, undefined)
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

	let lastPath: string | undefined = undefined
	async function replaceScript(lastEdit: LastEdit) {
		currentScript = lastEdit
		if (lastPath !== lastEdit.path) {
			// schema = emptySchema()
		}
		// try {
		// 	await inferArgs(lastEdit.language, lastEdit.content, schema)
		// 	schema = schema
		// 	lastPath = lastEdit.path
		// 	validCode = true
		// } catch (e) {
		// 	console.error(e)
		// 	validCode = false
		// }
	}
	let editor: SimpleEditor
</script>

<svelte:window on:keydown={onKeyDown} />

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job={testJob} />

<main class="h-screen w-full">
	<div class="h-full w-full grid grid-cols-2">
		<SimpleEditor
			bind:this={editor}
			code={JSON.stringify($flowStore, null, 4)}
			lang="json"
			on:change={(e) => {
				const code = e.detail
				try {
					$flowStore = JSON.parse(code)
				} catch (e) {
					console.error('issue parsing new change:', code, e)
				}
			}}
		/>
		<div class="flex flex-col h-full relative">
			<div class="flex justify-center pt-1 absolute right-2 top-2">
				<FlowPreviewButtons />
			</div>
			<Splitpanes horizontal class="h-full">
				<Pane size={33}>
					{#if $flowStore?.value?.modules}
						<FlowModuleSchemaMap
							disableHeader
							bind:modules={$flowStore.value.modules}
							on:change={() => editor?.setCode(JSON.stringify($flowStore, null, 4))}
							disableAi
							disableTutorials
						/>
					{:else}
						<div class="text-red-400 mt-20">Missing flow modules</div>
					{/if}
				</Pane>
				<Pane size={67}>
					<FlowEditorPanel />
				</Pane>
			</Splitpanes>
		</div>
	</div>
</main>
