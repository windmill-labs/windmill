<script lang="ts">
	import SqsTriggerEditorInner from './SqsTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'

	let { selectedTrigger, isFlow, path, edit, isDeployed = false } = $props()
	let sqsTriggerEditor: SqsTriggerEditorInner | undefined = $state(undefined)

	async function openSqsTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			sqsTriggerEditor?.openNew(isFlow, path)
		} else {
			sqsTriggerEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		sqsTriggerEditor && openSqsTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		SQS triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<SqsTriggerEditorInner
			bind:this={sqsTriggerEditor}
			useDrawer={false}
			hideTarget
			editMode={edit}
			preventSave={!isDeployed}
			hideTooltips={!isDeployed}
			useEditButton
			on:toggle-edit-mode
			on:update-config
			on:update
			on:delete
		>
			{#snippet description()}
				<Description link="https://www.windmill.dev/docs/core_concepts/sqs_triggers">
					SQS triggers allow you to execute scripts and flows in response to messages in an AWS SQS
					queue. They can be configured to filter messages based on message attributes.
				</Description>
				{#if !isDeployed}
					<Alert
						title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the SQS trigger`}
						type="info"
						size="xs"
					/>
				{/if}
			{/snippet}
		</SqsTriggerEditorInner>
	</div>
{/if}
