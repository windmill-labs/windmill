<script lang="ts">
	import { onMount, setContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { cubicOut } from 'svelte/easing'
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
	import Button from '../../common/button/Button.svelte'
	import { Unlock } from 'lucide-svelte'

	export let app: App
	export let appPath: string
	export let breakpoint: Writable<EditorBreakpoint>
	export let policy: Policy
	export let summary: string
	export let workspace: string
	export let isEditor: boolean
	export let context: Record<string, any>
	export let noBackend: boolean = false
	export let isLocked = false

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
		jobs: writable([]),
		staticExporter: writable({}),
		noBackend
	})

	let mounted = false

	onMount(() => {
		mounted = true
	})

	$: mounted && ($worldStore = buildWorld($staticOutputs, undefined, context))
	$: width = $breakpoint === 'sm' ? 'max-w-[640px]' : 'w-full '
	$: lockedClasses = isLocked ? '!max-h-[400px] overflow-hidden pointer-events-none' : ''
</script>

<div class="relative">
	<div class="{$$props.class} {lockedClasses} h-full max-h-[calc(100%-41px)] overflow-auto 
	w-full {app.fullscreen ? '' : 'max-w-6xl'} mx-auto">
		{#if $appStore.grid}
			<div class={classNames('mx-auto pb-4', width)}>
				<GridEditor {policy} />
			</div>
		{/if}
	</div>
	{#if isLocked}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div
			transition:fade|local={{ duration: 200, easing: cubicOut }}
			on:click={() => isLocked = false}
			class="absolute inset-0 center-center bg-black/20 z-50 backdrop-blur-[1px] cursor-pointer"
		>
			<Button on:click={() => isLocked = false}>
				Unlock preview
				<Unlock size={18} class="ml-1" strokeWidth={2.5} />
			</Button>
		</div>
	{/if}
</div>
