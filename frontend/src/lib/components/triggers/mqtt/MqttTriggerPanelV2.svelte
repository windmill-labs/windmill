<script lang="ts">
	import MqttTriggerEditorInner from './MqttTriggerEditorInner.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'

	let { selectedTrigger, isFlow, path, edit, isDeployed = false, isEditor } = $props()
	let mqttTriggerEditor: MqttTriggerEditorInner | undefined = $state(undefined)

	async function openMqttTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			mqttTriggerEditor?.openNew(isFlow, path)
		} else {
			mqttTriggerEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		mqttTriggerEditor && openMqttTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		MQTT triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<MqttTriggerEditorInner
			bind:this={mqttTriggerEditor}
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
				<Description link="https://www.windmill.dev/docs/core_concepts/mqtt_triggers">
					MQTT triggers allow you to execute scripts and flows in response to MQTT messages. They
					can be configured to subscribe to specific topics with different QoS levels.
				</Description>
				{#if !isDeployed}
					<Alert
						title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the MQTT trigger`}
						type="info"
						size="xs"
					/>
				{/if}
			{/snippet}
		</MqttTriggerEditorInner>
	</div>
{/if}
