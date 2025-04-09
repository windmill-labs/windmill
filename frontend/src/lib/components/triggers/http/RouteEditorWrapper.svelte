<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let { selectedTrigger, isFlow, currentPath, isEditing } = $props()

	function openRouteEditor(path: string, isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, currentPath)
		} else {
			routeEditor?.openEdit(path, isFlow, isEditing)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'routes' &&
			routeEditor &&
			openRouteEditor(selectedTrigger.path, isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

<RouteEditorInner
	useDrawer={false}
	bind:this={routeEditor}
	hideTarget
	on:update-config
	hidePath
	editMode={isEditing}
/>
