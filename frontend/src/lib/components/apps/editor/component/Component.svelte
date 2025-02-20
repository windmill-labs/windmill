<script lang="ts">
	import ComponentInner from './ComponentInner.svelte'
	import ComponentRendered from './ComponentRendered.svelte'
	import type { AppComponent } from './components'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let render: boolean
	export let fullHeight: boolean
	export let overlapped: string | undefined = undefined
	export let componentDraggedId: string | undefined = undefined

	export let moveMode: string | undefined = undefined
	let everRender = render

	$: render && !everRender && (everRender = true)
</script>

{#if everRender}
	<ComponentRendered
		{moveMode}
		{componentDraggedId}
		{overlapped}
		{render}
		{component}
		{selected}
		{locked}
		{fullHeight}
	/>
{:else}
	<ComponentInner
		{component}
		render={false}
		componentContainerHeight={0}
		errorHandledByComponent={false}
		inlineEditorOpened={false}
	/>
{/if}
