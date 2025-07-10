<script lang="ts">
	import { run } from 'svelte/legacy'

	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Plus, X } from 'lucide-svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import type { MqttClientVersion, MqttSubscribeTopic } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'
	import { fade } from 'svelte/transition'

	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import TestingBadge from '../testingBadge.svelte'

	interface Props {
		can_write?: boolean
		headless?: boolean
		mqtt_resource_path?: string
		subscribe_topics?: MqttSubscribeTopic[]
		client_version?: MqttClientVersion
		isValid?: boolean
		client_id?: string
		showTestingBadge?: boolean
	}

	let {
		can_write = false,
		headless = false,
		mqtt_resource_path = $bindable(''),
		subscribe_topics = $bindable([]),
		client_version = $bindable('v5'),
		isValid = $bindable(false),
		client_id = $bindable(''),
		showTestingBadge = false
	}: Props = $props()

	const isValidSubscribeTopics = (subscribe_topics: MqttSubscribeTopic[]): boolean => {
		if (
			subscribe_topics.length === 0 ||
			subscribe_topics.find((subscribe_topic) => emptyStringTrimmed(subscribe_topic.topic))
		) {
			return false
		}

		return true
	}
	run(() => {
		isValid = isValidSubscribeTopics(subscribe_topics) && !emptyStringTrimmed(mqtt_resource_path)
	})
</script>

<div class="flex flex-col gap-12">
	<Section label="MQTT" {headless}>
		{#snippet header()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		<div class="flex flex-col w-full gap-12">
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
									<!-- svelte-ignore a11y_label_has_associated_control -->
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
										<ToggleButtonGroup bind:selected={v.qos}>
											{#snippet children({ item })}
												<ToggleButton value={'qos0'} label="At most once (QoS 0)" {item} />
												<ToggleButton value={'qos1'} label="At least once (QoS 1)" {item} />
												<ToggleButton value={'qos2'} label="Exactly once (QoS 2)" {item} />
											{/snippet}
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
										bind:value={subscribe_topics[i].topic}
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
								onclick={() => {
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
		</div>
	</Section>
</div>
