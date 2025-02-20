<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'

	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Section from '$lib/components/Section.svelte'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'
	import MqttTriggerEditor from './MqttTriggerEditor.svelte'
	import { MqttTriggerService, type MqttTrigger } from '$lib/gen'
	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false
	export let isEditor: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	let mqttTriggerEditor: MqttTriggerEditor
	let openForm = true
	let dontCloseOnLoad = false

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'mqtt' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			mqttTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let mqttTriggers: (MqttTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			mqttTriggers = (
				await MqttTriggerService.listMqttTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), mqtt_count: mqttTriggers?.length }
			openForm = mqttTriggers?.length === 0 || dontCloseOnLoad
		} catch (e) {
			console.error('impossible to load mqtt triggers', e)
		}
	}
</script>

<MqttTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={mqttTriggerEditor}
/>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		MQTT triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description link="https://mqtt.org/">
			Windmill can connect to an MQTT broker and subscribes to specific topics thus allowing the
			execution of script/flows based on the event triggered by those subscribed topics
		</Description>

		{#if !newItem && mqttTriggers && mqttTriggers.length > 0}
			<Section label="MQTT">
				<div class="flex flex-col gap-4">
					<div class="flex flex-col divide-y pt-2">
						{#each mqttTriggers as mqttTriggers (mqttTriggers.path)}
							<div class="grid grid-cols-5 text-2xs items-center py-2">
								<div class="col-span-2 truncate">{mqttTriggers.path}</div>

								<div class="flex justify-end">
									<button
										on:click={() => mqttTriggerEditor?.openEdit(mqttTriggers.path, isFlow)}
										class="px-2"
									>
										{#if mqttTriggers.canWrite}
											Edit
										{:else}
											View
										{/if}
									</button>
								</div>
							</div>
						{/each}
					</div>
				</div>
			</Section>
		{/if}

		<TriggersEditorSection
			on:applyArgs
			on:saveTrigger={(e) => {
				mqttTriggerEditor?.openNew(isFlow, path, e.detail.config)
			}}
			on:addPreprocessor
			on:updateSchema
			on:testWithArgs
			cloudDisabled={false}
			triggerType="mqtt"
			{isFlow}
			{path}
			{isEditor}
			{canHavePreprocessor}
			{hasPreprocessor}
			{newItem}
			{openForm}
			bind:showCapture={dontCloseOnLoad}
		/>
	</div>
{/if}
