<script lang="ts">
	import { onMount, setContext } from 'svelte'

	import { writable } from 'svelte/store'
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

	export let app: App

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>('preview')
	const breakpoint = writable<EditorBreakpoint>('lg')

	const connectingInput = writable<ConnectingInput>({
		opened: false,
		input: undefined
	})

	const runnableComponents = writable<Record<string, () => void>>({})

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		app: appStore,
		selectedComponent,
		mode,
		connectingInput,
		breakpoint,
		runnableComponents
	})

	let mounted = false

	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs))
	$: width = $breakpoint === 'sm' ? 'w-[640px]' : 'w-full '
</script>

<div class="h-full w-full">
	{#if $appStore.grid}
		<div class={classNames('mx-auto h-full', width)}>
			<GridEditor />
		</div>
	{/if}
</div>
