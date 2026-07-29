<script lang="ts">
	import NatsTriggerEditorInner from './NatsTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { onMount } from 'svelte'
	import { enterpriseLicense } from '$lib/stores'

	let {
		selectedTrigger,
		isFlow,
		path,
		isDeployed = false,
		defaultValues = undefined,
		customLabel = undefined,
		...props
	} = $props()
	let natsTriggerEditor: NatsTriggerEditorInner | undefined = $state(undefined)

	async function openNatsTriggerEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await natsTriggerEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			natsTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		natsTriggerEditor && openNatsTriggerEditor(isFlow)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		NATS triggers are an enterprise only feature.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<NatsTriggerEditorInner
			bind:this={natsTriggerEditor}
			useDrawer={false}
			hideTarget
			hideTooltips={!isDeployed}
			useEditButton
			allowDraft={true}
			trigger={selectedTrigger}
			{customLabel}
			{isDeployed}
			{cloudDisabled}
			{...props}
		>
			{#snippet description()}
				{#if cloudDisabled}
					<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
						NATS triggers are disabled in the multi-tenant cloud.
					</Alert>
				{:else}
					<Description link="https://www.windmill.dev/docs/core_concepts/nats_triggers">
						NATS triggers allow you to execute scripts and flows in response to NATS messages. They
						can be configured to listen to specific subjects and to use JetStream or not.
					</Description>
				{/if}
			{/snippet}
		</NatsTriggerEditorInner>
	</div>
{/if}
