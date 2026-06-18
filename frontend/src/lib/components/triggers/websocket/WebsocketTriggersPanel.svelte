<script lang="ts">
	import WebsocketTriggerEditorInner from './WebsocketTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { onMount } from 'svelte'

	let {
		selectedTrigger,
		isFlow,
		path,
		isDeployed = false,
		defaultValues = undefined,
		customLabel = undefined,
		...restProps
	} = $props()
	let wsTriggerEditor: WebsocketTriggerEditorInner | undefined = $state(undefined)

	async function openWebsocketTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			wsTriggerEditor?.openNew(isFlow, path, defaultValues)
		} else {
			wsTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		openWebsocketTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

<div class="flex flex-col gap-4">
	<WebsocketTriggerEditorInner
		bind:this={wsTriggerEditor}
		useDrawer={false}
		hideTarget
		hideTooltips={!isDeployed || cloudDisabled}
		allowDraft={true}
		trigger={selectedTrigger}
		{customLabel}
		{isDeployed}
		{cloudDisabled}
		{...restProps}
	>
		{#snippet description()}
			{#if cloudDisabled}
				<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
					WebSocket triggers are disabled in the multi-tenant cloud.
				</Alert>
			{:else}
				<Description link="https://www.windmill.dev/docs/core_concepts/websocket_triggers">
					WebSocket triggers allow real-time bidirectional communication between your scripts/flows
					and external systems. Each trigger creates a unique WebSocket endpoint.
				</Description>
			{/if}
		{/snippet}
	</WebsocketTriggerEditorInner>
</div>
