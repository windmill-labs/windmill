<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Plus, X } from 'lucide-svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import type { SubscribeTopic } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'
	import { fade } from 'svelte/transition'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let mqtt_resource_path: string = ''
	export let subscribe_topics: SubscribeTopic[] = []
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let topics: string[] = []
	$: isValid = topics.length > 0
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
		</div>
	</Section>
</div>
