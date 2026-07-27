<script lang="ts">
	import AmqpTriggerEditorInner from './AmqpTriggerEditorInner.svelte'
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
		newDraft = false,
		customLabel = undefined,
		...props
	} = $props()
	let amqpTriggerEditor: AmqpTriggerEditorInner | undefined = $state(undefined)

	async function openAmqpTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			amqpTriggerEditor?.openNew(isFlow, path, defaultValues)
		} else {
			amqpTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		amqpTriggerEditor && openAmqpTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

<div class="flex flex-col gap-4">
	<AmqpTriggerEditorInner
		bind:this={amqpTriggerEditor}
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
					AMQP triggers are disabled in the multi-tenant cloud.
				</Alert>
			{:else}
				<Description link="https://www.windmill.dev/docs/core_concepts/amqp_triggers">
					AMQP triggers allow you to execute scripts and flows in response to messages consumed from
					an AMQP (RabbitMQ) queue.
				</Description>
			{/if}
		{/snippet}
	</AmqpTriggerEditorInner>
</div>
