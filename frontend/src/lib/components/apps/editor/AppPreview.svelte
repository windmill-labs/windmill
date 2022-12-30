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

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>('preview')

	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined
	})

	const runnableComponents = writable<Record<string, () => Promise<void>>>({})

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		lazyGrid: writable(app.grid),
		app: appStore,
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents,
		appPath
	})

	let mounted = false

	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs, undefined))
	$: width = $breakpoint === 'sm' ? 'w-[640px]' : 'w-full '
</script>

<div class="h-full w-5/6 mx-auto">
	{#if $appStore.grid}
		<div class={classNames('mx-auto h-full', width)}>
			<GridEditor {policy} />
		</div>
	{/if}
</div>
