<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Plus, X } from 'lucide-svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import type { MqttClientVersion, MqttV3Config, MqttV5Config, SubscribeTopic } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'
	import { fade } from 'svelte/transition'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { emptyStringTrimmed } from '$lib/utils'

	const DEFAULT_V5_CONFIG: MqttV5Config = {
		clean_start: true,
		keep_alive: undefined,
		session_expiration: undefined,
		receive_maximum: undefined,
		maximum_packet_size: undefined
	}

	const DEFAULT_V3_CONFIG: MqttV3Config = {
		clean_session: true
	}

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let mqtt_resource_path: string = ''
	export let subscribe_topics: SubscribeTopic[] = []
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let v3_config: MqttV3Config | undefined
	export let v5_config: MqttV5Config | undefined
	export let client_version: MqttClientVersion = 'v5'
	export let isValid: boolean = false
	export let client_id: string
	if (!v3_config) {
		v3_config = DEFAULT_V3_CONFIG
	} else {
		v3_config = {
			clean_session: v3_config.clean_session
		}
	}
	if (!v5_config) {
		v5_config = DEFAULT_V5_CONFIG
	} else {
		v5_config = {
			clean_start: v5_config.clean_start,
			keep_alive: v5_config.keep_alive,
			session_expiration: v5_config.session_expiration,
			receive_maximum: v5_config.receive_maximum,
			maximum_packet_size: v5_config.maximum_packet_size
		}
	}
	$: isValid = subscribe_topics.length > 0 && !emptyStringTrimmed(mqtt_resource_path)
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			captureType="mqtt"
			disabled={!isValid}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="MQTT" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Subsection label="Connection Setup">
				<ResourcePicker resourceType="mqtt" disabled={!can_write} bind:value={mqtt_resource_path} />
			</Subsection>

			<Subsection label="Topics">
				<p class="text-xs text-tertiary mb-2"
					>Choose which topics you want to subscribe to<Required required={true} />
				</p>
				<div class="flex flex-col gap-4 mt-1">
					{#each subscribe_topics as v, i}
						<div class="flex w-full gap-2 items-center">
							<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
								<div class="flex flex-row gap-2 w-full">
									<!-- svelte-ignore a11y-label-has-associated-control -->
									<label class="flex flex-col w-full gap-1">
										<div class="text-secondary text-sm">QoS</div>
										<ToggleButtonGroup bind:selected={v.qos}>
											<ToggleButton value={0} label="QoS 0" />
											<ToggleButton value={1} label="QoS 1" />
											<ToggleButton value={2} label="QoS 2" />
										</ToggleButtonGroup>
									</label>
								</div>

								<label class="flex flex-col w-full gap-1">
									<div class="text-secondary text-sm">Topic</div>
									<input
										type="text"
										bind:value={v.topic}
										disabled={!can_write}
										placeholder="topic"
										autocomplete="off"
									/>
								</label>
							</div>
							<button
								transition:fade|local={{ duration: 100 }}
								class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
								aria-label="Clear"
								on:click={() => {
									subscribe_topics = subscribe_topics.filter((_, index) => index !== i)
								}}
							>
								<X size={14} />
							</button>
						</div>
					{/each}

					<div class="flex items-baseline">
						<Button
							variant="border"
							color="light"
							size="xs"
							btnClasses="mt-1"
							on:click={() => {
								if (subscribe_topics == undefined || !Array.isArray(subscribe_topics)) {
									subscribe_topics = []
								}
								subscribe_topics = subscribe_topics.concat({
									qos: 0,
									topic: ''
								})
							}}
							startIcon={{ icon: Plus }}
						>
							Add topic
						</Button>
					</div>
				</div>
			</Subsection>

			<Subsection label="Advanced" collapsable={true}>
				<div class="flex p-2 flex-col gap-2 mt-3">
					<ToggleButtonGroup bind:selected={client_version}>
						<ToggleButton value="v5" label="Version 5" />
						<ToggleButton value="v3" label="Version 3" />
					</ToggleButtonGroup>

					<input
						type="text"
						bind:value={client_id}
						disabled={!can_write}
						placeholder="client id"
						autocomplete="off"
					/>

					{#if client_version === 'v5'}
						<Toggle
							textClass="font-normal text-sm"
							color="nord"
							size="xs"
							checked={v5_config.clean_start}
							on:change={() => {
								v5_config.clean_start = !v5_config.clean_start
							}}
							options={{
								right: 'Clean start',
								rightTooltip: '',
								rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/mqtt_trigger'
							}}
							class="py-1"
						/>

						<input
							type="number"
							min={10}
							bind:value={v5_config.keep_alive}
							disabled={!can_write}
							placeholder="keep alive"
							autocomplete="off"
						/>

						<input
							type="number"
							min={10}
							bind:value={v5_config.session_expiration}
							disabled={!can_write}
							placeholder="session expiration"
							autocomplete="off"
						/>

						<input
							type="number"
							min={10}
							bind:value={v5_config.receive_maximum}
							disabled={!can_write}
							placeholder="receive maximum"
							autocomplete="off"
						/>

						<input
							type="number"
							min={10}
							bind:value={v5_config.maximum_packet_size}
							disabled={!can_write}
							placeholder="maximum packet_size"
							autocomplete="off"
						/>
					{:else if client_version === 'v3'}
						<Toggle
							textClass="font-normal text-sm"
							color="nord"
							size="xs"
							checked={v3_config.clean_session}
							on:change={() => {
								v3_config.clean_session = !v3_config.clean_session
							}}
							options={{
								right: 'Clean session',
								rightTooltip: '',
								rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/mqtt_trigger'
							}}
							class="py-1"
						/>
					{/if}
				</div>
			</Subsection>
		</div>
	</Section>
</div>
