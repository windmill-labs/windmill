<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { userStore } from '$lib/stores'
	import { Alert } from '$lib/components/common'
	import { onMount } from 'svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let {
		selectedTrigger,
		isFlow,
		path,
		defaultValues = undefined,
		isEditor = false,
		customLabel = undefined,
		...restProps
	} = $props()

	async function openRouteEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await routeEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			routeEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		if (routeEditor) {
			openRouteEditor(isFlow)
		}
	})
</script>

<RouteEditorInner
	useDrawer={false}
	bind:this={routeEditor}
	hideTarget
	{isEditor}
	{customLabel}
	trigger={selectedTrigger}
	allowDraft
	{...restProps}
>
	{#snippet description()}
		<div class="flex flex-col gap-2 pb-4">
			<Description link="https://www.windmill.dev/docs/core_concepts/http_routing"
				>Routes expose your scripts and flows as HTTP endpoints. Each route can be configured with a
				specific HTTP method and path.</Description
			>

			{#if !$userStore?.is_admin && !$userStore?.is_super_admin && selectedTrigger.isDraft}
				<Alert title="Non-admin users are limited to workspaced routes" type="info" size="xs" />
			{/if}
		</div>
	{/snippet}
</RouteEditorInner>
