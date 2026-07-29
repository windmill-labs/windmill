<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import GcpTriggerEditorInner from './GcpTriggerEditorInner.svelte'
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
	let gcpTriggerEditor: GcpTriggerEditorInner | undefined = $state(undefined)

	async function openGcpTriggerEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await gcpTriggerEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			gcpTriggerEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		gcpTriggerEditor && openGcpTriggerEditor(isFlow)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		GCP Pub/Sub triggers are an enterprise only feature.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<GcpTriggerEditorInner
			bind:this={gcpTriggerEditor}
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
						GCP Pub/Sub triggers are disabled in the multi-tenant cloud.
					</Alert>
				{:else}
					<Description link="https://www.windmill.dev/docs/core_concepts/gcp_triggers">
						GCP Pub/Sub triggers execute scripts and flows in response to messages published to
						Google Cloud Pub/Sub topics.
					</Description>
				{/if}
			{/snippet}
		</GcpTriggerEditorInner>
	</div>
{/if}
