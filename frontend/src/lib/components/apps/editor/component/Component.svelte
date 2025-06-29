<script lang="ts">
	import { getContext } from 'svelte'
	import ComponentInner from './ComponentInner.svelte'
	import ComponentRendered from './ComponentRendered.svelte'
	import type { AppComponent } from './components'
	import type { AppViewerContext } from '../../types'

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	interface Props {
		component: AppComponent
		selected: boolean
		locked?: boolean
		render: boolean
		fullHeight: boolean
		overlapped?: string | undefined
		componentDraggedId?: string | undefined
	}

	let {
		component,
		selected,
		locked = false,
		render,
		fullHeight,
		overlapped = undefined,
		componentDraggedId = undefined
	}: Props = $props()
	let everRender = $state(render)

	$effect(() => {
		render && !everRender && (everRender = true)
	})
</script>

{#if everRender || $app.eagerRendering}
	<ComponentRendered
		on:expand
		on:lock
		on:fillHeight
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
