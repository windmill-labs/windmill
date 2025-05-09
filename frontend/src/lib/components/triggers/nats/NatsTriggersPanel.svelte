<script lang="ts">
	import NatsTriggerEditorInner from './NatsTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { onMount } from 'svelte'

	let {
		selectedTrigger,
		isFlow,
		path,
		edit,
		isDeployed = false,
		isEditor,
		defaultValues = undefined,
		newDraft = false,
		customLabel = undefined
	} = $props()
	let natsTriggerEditor: NatsTriggerEditorInner | undefined = $state(undefined)

	async function openNatsTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			natsTriggerEditor?.openNew(isFlow, path, defaultValues, newDraft)
		} else {
			natsTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		natsTriggerEditor && openNatsTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		NATS triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<NatsTriggerEditorInner
			bind:this={natsTriggerEditor}
			useDrawer={false}
			hideTarget
			editMode={edit}
			hideTooltips={!isDeployed}
			useEditButton
			allowDraft={true}
			hasDraft={!!selectedTrigger.draftConfig}
			{isEditor}
			isDraftOnly={selectedTrigger.isDraft}
			on:toggle-edit-mode
			on:update-config
			on:update
			on:delete
			on:save-draft
			on:reset
			{customLabel}
		>
			{#snippet description()}
				<Description link="https://www.windmill.dev/docs/core_concepts/nats_triggers">
					NATS triggers allow you to execute scripts and flows in response to NATS messages. They
					can be configured to listen to specific subjects and to use JetStream or not.
				</Description>
			{/snippet}
		</NatsTriggerEditorInner>
	</div>
{/if}
