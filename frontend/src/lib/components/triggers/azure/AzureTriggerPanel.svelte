<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import AzureTriggerEditorInner from './AzureTriggerEditorInner.svelte'
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
	let azureTriggerEditor: AzureTriggerEditorInner | undefined = $state(undefined)

	async function openAzureTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			azureTriggerEditor?.openNew(isFlow, path, defaultValues)
		} else {
			azureTriggerEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		azureTriggerEditor && openAzureTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		Azure Event Grid triggers are an enterprise only feature.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<AzureTriggerEditorInner
			bind:this={azureTriggerEditor}
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
						Azure Event Grid triggers are disabled in the multi-tenant cloud.
					</Alert>
				{:else}
					<Description link="https://www.windmill.dev/docs/core_concepts/azure_triggers">
						Azure Event Grid triggers execute scripts and flows in response to events from Azure
						Event Grid (basic) or Event Grid Namespaces (push or pull).
					</Description>
				{/if}
			{/snippet}
		</AzureTriggerEditorInner>
	</div>
{/if}
