<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let { selectedTrigger, isFlow, path, edit = false } = $props()

	async function openRouteEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, path)
		} else {
			routeEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'http' &&
			routeEditor &&
			openRouteEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

<RouteEditorInner
	useDrawer={false}
	bind:this={routeEditor}
	hideTarget
	useEditButton
	on:update-config
	on:update
	showCapture
	editMode={edit}
	on:toggle-edit-mode
>
	{#snippet description()}
		<Description link="https://www.windmill.dev/docs/core_concepts/http_routing" class="mb-4"
			>Routes expose your scripts and flows as HTTP endpoints. Each route can be configured with a
			specific HTTP method and path.</Description
		>
	{/snippet}
</RouteEditorInner>
