<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { selectId } from '../../utils'

	export let id: string
	export let type: string

	const { connectingInput, selectedComponent, app, focusedGrid } =
		getContext<AppViewerContext>('AppViewerContext')

	const { manuallyOpened } = getContext<ContextPanelContext>('ContextPanel')

	function selectComponent(e: PointerEvent, id: string) {
		if (!$connectingInput.opened) {
			selectId(e, id, selectedComponent, $app)
			if ($focusedGrid?.parentComponentId != id) {
				$focusedGrid = undefined
			}
		}
	}

	// Prevent interaction with the component when connecting an input
	// We let the event bubble up if the component is a container, so we can select a tab for example
	function preventInteraction(event: Event, isContainer: boolean = false) {
		if ($connectingInput.opened && !isContainer) {
			event.stopPropagation()
		}
	}

	function onPointerDown(e: PointerEvent) {
		if (!$connectingInput.opened) {
			selectComponent(e, id)
		} else {
			$manuallyOpened[id] = true
			e.stopPropagation()
		}
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	class={$$props.class}
	on:pointerdown={onPointerDown}
	on:click|capture={(event) => preventInteraction(event, type === 'tabscomponent')}
	on:drag|capture={preventInteraction}
	on:pointerup|capture={(event) => preventInteraction(event, type === 'tabscomponent')}
>
	<slot />
</div>
