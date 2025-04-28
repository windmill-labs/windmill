<script lang="ts">
	import NatsTriggerEditorInner from './NatsTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'

	let { selectedTrigger, isFlow, path, edit, isDeployed = false, isEditor } = $props()
	let natsTriggerEditor: NatsTriggerEditorInner | undefined = $state(undefined)

	async function openNatsTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			natsTriggerEditor?.openNew(isFlow, path)
		} else {
			natsTriggerEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
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
			preventSave={!isDeployed}
			hideTooltips={!isDeployed}
			useEditButton
			{isEditor}
			on:toggle-edit-mode
			on:update-config
			on:update
		>
			{#snippet description()}
				<Description link="https://www.windmill.dev/docs/core_concepts/nats_triggers">
					NATS triggers allow you to execute scripts and flows in response to NATS messages. They
					can be configured to listen to specific subjects and to use JetStream or not.
				</Description>
				{#if !isDeployed}
					<Alert
						title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the NATS trigger`}
						type="info"
						size="xs"
					/>
				{/if}
			{/snippet}
		</NatsTriggerEditorInner>
	</div>
{/if}
