<script lang="ts">
	import { onMount, setContext } from 'svelte'

	import { writable, type Writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type {
		App,
		AppEditorContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode
	} from '../types'
	import GridEditor from './GridEditor.svelte'

	import { classNames } from '$lib/utils'
	import type { Policy } from '$lib/gen'

	export let app: App
	export let appPath: string
	export let breakpoint: Writable<EditorBreakpoint>
	export let policy: Policy
	export let summary: string
	export let workspace: string
	export let isEditor: boolean
	export let context: Record<string, any>

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>('preview')

	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined,
		hoveredComponent: undefined
	})

	const runnableComponents = writable<Record<string, () => Promise<void>>>({})

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		lazyGrid: writable(app.grid),
		app: appStore,
		summary: writable(summary),
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents,
		appPath,
		workspace,
		onchange: undefined,
		isEditor,
		jobs: writable([])
	})

	let mounted = false

	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs, undefined, context))
	$: width = $breakpoint === 'sm' ? 'max-w-[640px]' : 'w-full '
</script>

<div class="h-full w-full {app.fullscreen ? '' : 'max-w-6xl'} mx-auto">
	{#if $appStore.grid}
		<div class={classNames('mx-auto pb-4', width)}>
			<GridEditor {policy} />
		</div>
	{/if}
</div>
