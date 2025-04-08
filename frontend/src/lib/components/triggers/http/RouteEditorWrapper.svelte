<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let { selectedTrigger, isFlow, currentPath } = $props()

	function openRouteEditor(path: string, isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, currentPath)
		} else {
			routeEditor?.openEdit(path, isFlow)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'routes' &&
			routeEditor &&
			openRouteEditor(selectedTrigger.path, isFlow, selectedTrigger.isDraft ?? false)
	})

	$inspect('dbg inspect', selectedTrigger?.path)
</script>

<RouteEditorInner useDrawer={false} bind:this={routeEditor} hideTarget on:update-config />
