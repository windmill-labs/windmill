<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let { selectedTrigger, isFlow, currentPath, header = undefined } = $props()

	function openRouteEditor(path: string, isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, currentPath)
		} else {
			routeEditor?.openEdit(path, isFlow, false)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'http' &&
			routeEditor &&
			openRouteEditor(selectedTrigger.path, isFlow, selectedTrigger.isDraft ?? false)
	})

	$inspect('dbg inspect', selectedTrigger?.path)
</script>

<RouteEditorInner
	useDrawer={false}
	bind:this={routeEditor}
	hideTarget
	hidePath
	on:update-config
	on:update
	{header}
/>
