<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Plus, X } from 'lucide-svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import type { MqttClientVersion, MqttV3Config, MqttV5Config, MqttSubscribeTopic } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'
	import { fade } from 'svelte/transition'

	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import { DEFAULT_V3_CONFIG, DEFAULT_V5_CONFIG } from './constant'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let mqtt_resource_path: string = ''
	export let subscribe_topics: MqttSubscribeTopic[] = []
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let v3_config: MqttV3Config = DEFAULT_V3_CONFIG
	export let v5_config: MqttV5Config = DEFAULT_V5_CONFIG
	export let client_version: MqttClientVersion = 'v5'
	export let isValid: boolean = false
	export let client_id: string = ''

	const activateV5Options = {
		topic_alias: Boolean(v5_config.topic_alias),
		session_expiry_interval: Boolean(v5_config.session_expiry_interval)
	}

	const isValidSubscribeTopics = (subscribe_topics: MqttSubscribeTopic[]): boolean => {
		if (
			subscribe_topics.length === 0 ||
			subscribe_topics.find((subscribe_topic) => emptyStringTrimmed(subscribe_topic.topic))
		) {
			return false
		}

		return true
	}
	$: isValid = isValidSubscribeTopics(subscribe_topics) && !emptyStringTrimmed(mqtt_resource_path)
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
			<Subsection label="Connection setup">
				<ResourcePicker resourceType="mqtt" disabled={!can_write} bind:value={mqtt_resource_path} />
				{#if !emptyStringTrimmed(mqtt_resource_path)}
					<TestTriggerConnection kind="mqtt" args={{ mqtt_resource_path, client_version }} />
				{/if}
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
										<div class="flex gap-2 mb-1">
											<span class="text-secondary text-sm">
												QoS
												<Tooltip
													documentationLink="https://www.windmill.dev/docs/core_concepts/mqtt_triggers#configure-topic-subscriptions"
													><ul class="list-disc list-inside space-y-2">
														<li>
															<span class="font-bold">QoS 0 - At most once</span>
															<ul class="list-none ml-4 text-sm">
																<li>❌ Messages may be lost if the network fails.</li>
																<li>✅ No duplicates, but no delivery guarantee.</li>
																<li
																	>⚠️ Scripts/flows may not be triggered if the message is lost.</li
																>
															</ul>
														</li>
														<li>
															<span class="font-bold">QoS 1 - At least once</span>
															<ul class="list-none ml-4 text-sm">
																<li>✅ Guaranteed to receive the message at least once.</li>
																<li>❌ May receive duplicate messages.</li>
																<li
																	>⚠️ Scripts/flows may be triggered more than once for the same
																	message.</li
																>
															</ul>
														</li>
														<li>
															<span class="font-bold">QoS 2 - Exactly once</span>
															<ul class="list-none ml-4 text-sm">
																<li>✅ Ensures each message is received only once.</li>
																<li>❌ Slightly slower due to extra processing.</li>
																<li
																	>✅ Scripts/flows will be triggered exactly once for each message.</li
																>
															</ul>
														</li>
													</ul></Tooltip
												>
											</span>
										</div>
										<ToggleButtonGroup bind:selected={v.qos} let:item>
											<ToggleButton value={'qos0'} label="At most once (QoS 0)" {item} />
											<ToggleButton value={'qos1'} label="At least once (QoS 1)" {item} />
											<ToggleButton value={'qos2'} label="Exactly once (QoS 2)" {item} />
										</ToggleButtonGroup>
									</label>
								</div>

								<label class="flex flex-col w-full gap-1">
									<div class="flex gap-2 mb-1">
										<span class="text-secondary text-sm">
											Topic
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/mqtt_triggers#configure-topic-subscriptions"
												>The topic you want to subscribe to</Tooltip
											>
										</span>
									</div>
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
								subscribe_topics = [
									...subscribe_topics,
									{
										qos: 'qos0',
										topic: ''
									}
								]
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
					<ToggleButtonGroup bind:selected={client_version} let:item>
						<ToggleButton value="v5" label="Version 5" {item} />
						<ToggleButton value="v3" label="Version 3" {item} />
					</ToggleButtonGroup>

					<input
						type="text"
						bind:value={client_id}
						disabled={!can_write}
						placeholder="Client id"
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
								rightTooltip:
									'Start a new session without any stored messages or subscriptions if enabled. Otherwise, resume the previous session with stored subscriptions and undelivered messages. The default setting is 0.',
								rightDocumentationLink:
									'https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901039'
							}}
							class="py-1"
						/>

						<div class="flex flex-col gap-2">
							<Toggle
								textClass="font-normal text-sm"
								color="nord"
								size="xs"
								checked={activateV5Options.session_expiry_interval}
								on:change={() => {
									activateV5Options.session_expiry_interval =
										!activateV5Options.session_expiry_interval
									if (!activateV5Options.session_expiry_interval) {
										v5_config.session_expiry_interval = undefined
									}
								}}
								options={{
									right: 'Session expiry interval',
									rightTooltip:
										'Defines the time in seconds that the broker will retain the session after disconnection. If set to 0, the session ends immediately. If set to 4,294,967,295, the session will be retained indefinitely. Otherwise, subscriptions and undelivered messages are stored until the interval expires.',
									rightDocumentationLink: ''
								}}
								class="py-1"
							/>

							{#if activateV5Options.session_expiry_interval}
								<input
									type="number"
									bind:value={v5_config.session_expiry_interval}
									disabled={!can_write}
									placeholder="Session expiry interval"
									autocomplete="off"
								/>
							{/if}
						</div>

						<div class="flex flex-col gap-2">
							<Toggle
								textClass="font-normal text-sm"
								color="nord"
								size="xs"
								checked={activateV5Options.topic_alias}
								on:change={() => {
									activateV5Options.topic_alias = !activateV5Options.topic_alias
									if (!activateV5Options.topic_alias) {
										v5_config.topic_alias = undefined
									}
								}}
								options={{
									right: 'Topic alias maximum',
									rightTooltip:
										'Defines the maximum topic alias value the client will accept from the broker. A value of 0 indicates that topic aliases are not supported. The default value is 65536, which is the maximum allowed topic alias.',
									rightDocumentationLink:
										'https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901051'
								}}
								class="py-1"
							/>

							{#if activateV5Options.topic_alias}
								<input
									type="number"
									bind:value={v5_config.topic_alias}
									disabled={!can_write}
									placeholder="Topic alias"
									autocomplete="off"
								/>
							{/if}
						</div>
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
								rightTooltip:
									'Starts a new session without any stored messages or subscriptions if enabled. Otherwise, it resumes the previous session with stored subscriptions and undelivered messages. The default value is 0',
								rightDocumentationLink: ''
							}}
							class="py-1"
						/>
					{/if}
				</div>
			</Subsection>
		</div>
	</Section>
</div>
