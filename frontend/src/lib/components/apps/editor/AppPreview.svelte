<script lang="ts">
	import { onMount, setContext } from 'svelte'

	import { writable } from 'svelte/store'
	import { buildWorld, type World } from '../rx'
	import type {
		App,
		AppEditorContext,
		ConnectingInput,
		EditorBreakpoint,
		EditorMode,
		InputType
	} from '../types'
	import GridEditor from './GridEditor.svelte'

	import { classNames } from '$lib/utils'

	export let app: App
	export let initialMode: EditorMode = 'dnd'

	const appStore = writable<App>(app)
	const worldStore = writable<World | undefined>(undefined)
	const staticOutputs = writable<Record<string, string[]>>({})
	const selectedComponent = writable<string | undefined>(undefined)
	const mode = writable<EditorMode>(initialMode)
	const breakpoint = writable<EditorBreakpoint>('lg')

	const connectingInput = writable<ConnectingInput<InputType, any>>({
		opened: false,
		input: undefined
	})

	setContext<AppEditorContext>('AppEditorContext', {
		worldStore,
		staticOutputs,
		app: appStore,
		selectedComponent,
		mode,
		connectingInput,
		breakpoint
	})

	function clearSelectionOnPreview() {
		if ($mode === 'preview') {
			$selectedComponent = undefined
		}
	}

	let mounted = false

	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs))
	$: $mode && $selectedComponent && clearSelectionOnPreview()
	$: width = $breakpoint === 'sm' ? 'w-[640px]' : 'w-full '
</script>

<div class="h-full w-full">
	{#if $appStore.grid}
		<div class={classNames('mx-auto h-full', width)}>
			<GridEditor />
		</div>
	{/if}
</div>
