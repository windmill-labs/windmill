<script lang="ts">
	import KafkaTriggerEditorInner from './KafkaTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import { enterpriseLicense } from '$lib/stores'
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
	let kafkaTriggerEditor: KafkaTriggerEditorInner | undefined = $state(undefined)

	async function openKafkaTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			kafkaTriggerEditor?.openNew(isFlow, path, defaultValues, newDraft)
		} else {
			kafkaTriggerEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		openKafkaTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		Kafka triggers are an enterprise only feature.
	</Alert>
{:else if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		Kafka triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<KafkaTriggerEditorInner
			bind:this={kafkaTriggerEditor}
			useDrawer={false}
			hideTarget
			editMode={edit}
			hideTooltips={!isDeployed}
			allowDraft={true}
			hasDraft={!!selectedTrigger.draftConfig}
			isDraftOnly={selectedTrigger.isDraft}
			{isEditor}
			{customLabel}
			on:toggle-edit-mode
			on:update-config
			on:update
			on:delete
			on:save-draft
			on:reset
		>
			{#snippet description()}
				<Description link="https://www.windmill.dev/docs/core_concepts/kafka_triggers">
					Kafka triggers execute scripts and flows in response to messages published to Kafka
					topics.
				</Description>
				{#if !isDeployed}
					<Alert
						title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the kafka trigger`}
						type="info"
						size="xs"
					/>
				{/if}
			{/snippet}
		</KafkaTriggerEditorInner>
	</div>
{/if}
