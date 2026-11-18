<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { Alert } from '$lib/components/common'
	import { onMount } from 'svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)

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

	async function openRouteEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, path, defaultValues)
		} else {
			routeEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		if (routeEditor) {
			openRouteEditor(isFlow, selectedTrigger.isDraft ?? false)
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

			{#if !actingUser?.is_admin && !actingUser?.is_super_admin && selectedTrigger.isDraft}
				<Alert title="Non-admin users are limited to workspaced routes" type="info" size="xs" />
			{/if}
		</div>
	{/snippet}
</RouteEditorInner>
