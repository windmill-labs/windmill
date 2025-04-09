<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let { selectedTrigger, isFlow, path, header = undefined } = $props()

	function openRouteEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, path)
		} else {
			routeEditor?.openEdit(selectedTrigger.path, isFlow, false)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'http' &&
			routeEditor &&
			openRouteEditor(isFlow, selectedTrigger.isDraft ?? false)
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
