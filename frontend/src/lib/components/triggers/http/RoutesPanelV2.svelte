<script lang="ts">
	import RouteEditorInner from './RouteEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { userStore } from '$lib/stores'
	import { Alert } from '$lib/components/common'
	import TriggerLabel from '../TriggerLabel.svelte'

	let routeEditor = $state<RouteEditorInner | null>(null)
	let {
		selectedTrigger,
		isFlow,
		path,
		edit = false,
		isDeployed = false,
		small = false,
		defaultValues = undefined
	} = $props()

	async function openRouteEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			routeEditor?.openNew(isFlow, path, defaultValues)
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
	on:delete
	preventSave={!isDeployed}
	customLabel={small ? customLabel : undefined}
>
	{#snippet description()}
		<div class="flex flex-col gap-2 pb-4">
			<Description link="https://www.windmill.dev/docs/core_concepts/http_routing"
				>Routes expose your scripts and flows as HTTP endpoints. Each route can be configured with a
				specific HTTP method and path.</Description
			>

			{#if !$userStore?.is_admin && !$userStore?.is_super_admin && selectedTrigger.isDraft}
				<Alert title="Only workspace admins can create routes" type="info" size="xs" />
			{:else if !isDeployed}
				<Alert
					title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the route`}
					type="info"
					size="xs"
				/>
			{/if}
		</div>
	{/snippet}
</RouteEditorInner>

{#snippet customLabel()}
	<TriggerLabel trigger={selectedTrigger} />
{/snippet}
