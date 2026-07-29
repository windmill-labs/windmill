<script lang="ts">
	import SqsTriggerEditorInner from './SqsTriggerEditorInner.svelte'
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
		newDraft = false,
		customLabel = undefined,
		...props
	} = $props()
	let sqsTriggerEditor: SqsTriggerEditorInner | undefined = $state(undefined)

	async function openSqsTriggerEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await sqsTriggerEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			sqsTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		sqsTriggerEditor && openSqsTriggerEditor(isFlow)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

{#if !$enterpriseLicense}
	<Alert title="EE Only" type="warning" size="xs">
		SQS triggers are an enterprise only feature.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<SqsTriggerEditorInner
			bind:this={sqsTriggerEditor}
			useDrawer={false}
			hideTarget
			hideTooltips={!isDeployed || cloudDisabled}
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
						SQS triggers are disabled in the multi-tenant cloud.
					</Alert>
				{:else}
					<Description link="https://www.windmill.dev/docs/core_concepts/sqs_triggers">
						SQS triggers allow you to execute scripts and flows in response to messages in an AWS
						SQS queue. They can be configured to filter messages based on message attributes.
					</Description>
				{/if}
			{/snippet}
		</SqsTriggerEditorInner>
	</div>
{/if}
