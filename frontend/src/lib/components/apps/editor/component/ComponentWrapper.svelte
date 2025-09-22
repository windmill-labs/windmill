<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { dfs, selectId } from '../appUtils'

	interface Props {
		id: string
		type: string
		class: string
		children?: import('svelte').Snippet
	}

	let { id, type, class: clazz, children }: Props = $props()

	const { app, connectingInput, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	const { manuallyOpened } = getContext<ContextPanelContext>('ContextPanel')

	function selectComponent(e: PointerEvent, id: string) {
		if (!$connectingInput.opened) {
			selectId(e, id, selectedComponent, $app)
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
			const allIdsInPath = dfs($app.grid, id, $app.subgrids ?? {}) ?? []

			allIdsInPath.forEach((id) => {
				$manuallyOpened[id] = true
			})

			e.stopPropagation()
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_mouse_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={clazz}
	onpointerover={(e) => {
		if ($connectingInput.opened && $connectingInput.hoveredComponent !== id) {
			$connectingInput.hoveredComponent = id
		}
		e.stopPropagation()
	}}
	onpointerleave={(e) => {
		if ($connectingInput.opened) {
			$connectingInput.hoveredComponent = undefined
			e.stopPropagation()
		}
	}}
	onfocus={(e) => {
		if ($connectingInput.opened) {
			e.stopPropagation()
		}
	}}
	onpointerdown={onPointerDown}
	onclickcapture={(event) =>
		preventInteraction(event, type === 'tabscomponent' || type === 'steppercomponent')}
	ondragcapture={preventInteraction}
	onpointerupcapture={(event) =>
		preventInteraction(event, type === 'tabscomponent' || type === 'steppercomponent')}
>
	{@render children?.()}
</div>
