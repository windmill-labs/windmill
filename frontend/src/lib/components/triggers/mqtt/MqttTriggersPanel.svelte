<script lang="ts">
	import MqttTriggerEditorInner from './MqttTriggerEditorInner.svelte'
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
	let mqttTriggerEditor: MqttTriggerEditorInner | undefined = $state(undefined)

	async function openMqttTriggerEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await mqttTriggerEditor?.openNew(isFlow, (selectedTrigger.newTriggerSeed?.script_path ?? path), { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			mqttTriggerEditor?.openEdit(selectedTrigger.path, isFlow, selectedTrigger.draftConfig)
		}
	}

	onMount(() => {
		mqttTriggerEditor && openMqttTriggerEditor(isFlow)
	})

	const cloudDisabled = $derived(isCloudHosted())
</script>

<div class="flex flex-col gap-4">
	<MqttTriggerEditorInner
		bind:this={mqttTriggerEditor}
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
					MQTT triggers are disabled in the multi-tenant cloud.
				</Alert>
			{:else}
				<Description link="https://www.windmill.dev/docs/core_concepts/mqtt_triggers">
					MQTT triggers allow you to execute scripts and flows in response to MQTT messages. They
					can be configured to subscribe to specific topics with different QoS levels.
				</Description>
			{/if}
		{/snippet}
	</MqttTriggerEditorInner>
</div>
