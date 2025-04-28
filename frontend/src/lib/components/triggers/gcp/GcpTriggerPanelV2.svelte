<script lang="ts">
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { enterpriseLicense } from '$lib/stores'
	// Use the same approach as other panels
	import GcpTriggerEditorInner from './GcpTriggerEditorInner.svelte'

	let { selectedTrigger, isFlow, path, edit, isDeployed = false, isEditor } = $props()
	let gcpTriggerEditor: GcpTriggerEditorInner | undefined = $state(undefined)

	async function openGcpTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			gcpTriggerEditor?.openNew(isFlow, path)
		} else {
			gcpTriggerEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		gcpTriggerEditor && openGcpTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		GCP Pub/Sub triggers are an enterprise only feature.
	</Alert>
{:else if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		GCP Pub/Sub triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<GcpTriggerEditorInner
			bind:this={gcpTriggerEditor}
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
				<Description link="https://www.windmill.dev/docs/core_concepts/gcp_triggers">
					GCP Pub/Sub triggers execute scripts and flows in response to messages published to Google
					Cloud Pub/Sub topics.
				</Description>
				{#if !isDeployed}
					<Alert
						title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the GCP Pub/Sub trigger`}
						type="info"
						size="xs"
					/>
				{/if}
			{/snippet}
		</GcpTriggerEditorInner>
	</div>
{/if}
