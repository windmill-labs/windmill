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

	async function openWebsocketTriggerEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await wsTriggerEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			wsTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		openWebsocketTriggerEditor(isFlow)
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
