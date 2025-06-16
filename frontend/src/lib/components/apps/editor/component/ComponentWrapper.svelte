<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { dfs, selectId } from '../appUtils'

	export let id: string
	export let type: string

	const { app, connectingInput, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	const { manuallyOpened } = getContext<ContextPanelContext>('ContextPanel')

	function selectComponent(e: PointerEvent, id: string) {
		if (!$connectingInput.opened) {
			selectId(e, id, selectedComponent, app.val)
		}
	}

	// Prevent interaction with the component when connecting an input
	// We let the event bubble up if the component is a container, so we can select a tab for example
	function preventInteraction(event: Event, isContainer: boolean = false) {
		if ($connectingInput.opened && !isContainer && event.type != 'click') {
			event.stopPropagation()
		}
	}

	function onPointerDown(e: PointerEvent) {
		if (!$connectingInput.opened) {
			selectComponent(e, id)
		} else {
			const allIdsInPath = dfs(app.val.grid, id, app.val.subgrids ?? {}) ?? []

			allIdsInPath.forEach((id) => {
				$manuallyOpened[id] = true
			})

			e.stopPropagation()
		}
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-mouse-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={$$props.class}
	on:pointerover={(e) => {
		if ($connectingInput.opened && $connectingInput.hoveredComponent !== id) {
			$connectingInput.hoveredComponent = id
		}
		e.stopPropagation()
	}}
	on:pointerleave={(e) => {
		if ($connectingInput.opened) {
			$connectingInput.hoveredComponent = undefined
			e.stopPropagation()
		}
	}}
	on:focus={(e) => {
		if ($connectingInput.opened) {
			e.stopPropagation()
		}
	}}
	on:pointerdown={onPointerDown}
	on:click|capture={(event) =>
		preventInteraction(event, type === 'tabscomponent' || type === 'steppercomponent')}
	on:drag|capture={preventInteraction}
	on:pointerup|capture={(event) =>
		preventInteraction(event, type === 'tabscomponent' || type === 'steppercomponent')}
>
	<slot />
</div>
