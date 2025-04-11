<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { ChevronDown, Save } from 'lucide-svelte'
	import CaptureWrapper from './CaptureWrapper.svelte'
	import { capitalize } from '$lib/utils'
	import Popover from '../Popover.svelte'
	import Section from '../Section.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersWrapper from './TriggersWrapper.svelte'
	import { twMerge } from 'tailwind-merge'

	export let cloudDisabled: boolean
	export let triggerType: CaptureTriggerKind
	export let isFlow: boolean = false
	export let data: any = {}
	export let noSave = false
	export let isEditor: boolean = false
	export let path: string = ''
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false
	export let newItem: boolean
	export let openForm: boolean = false
	export let alwaysOpened: boolean = false
	export let showCapture: boolean = false

	const captureTypeLabels: Record<CaptureTriggerKind, string> = {
		http: 'custom HTTP route',
		websocket: 'WebSocket trigger',
		webhook: 'Webhook',
		kafka: 'Kafka trigger',
		email: 'Email trigger',
		nats: 'NATS trigger',
		mqtt: 'MQTT trigger',
		sqs: 'SQS trigger',
		postgres: 'Postgres trigger',
		gcp: 'GCP Pub/Sub trigger'
	}

	const { captureOn } = getContext<TriggerContext>('TriggerContext')

	let args: Record<string, any> = {}
	$: collapsed = !openForm

	const dispatch = createEventDispatcher()

	let init = false
	$: updateShowCapture(!!$captureOn)
	function updateShowCapture(show: boolean) {
		if (show && !init) {
			$captureOn = undefined
			showCapture = true
			init = true
		}
	}
</script>

<Section
	label={`+ New ${captureTypeLabels[triggerType]}`}
	collapsable={!alwaysOpened}
	bind:collapsed
>
	<svelte:fragment slot="action">
		{#if !collapsed || alwaysOpened}
			<div class="flex flex-row grow w-min-0 gap-2 items-center justify-end">
				{#if isEditor}
					<Button
						size="xs2"
						on:click={() => {
							showCapture = !showCapture
						}}
						variant="border"
						color="light"
						endIcon={{
							icon: ChevronDown,
							classes: twMerge('transition', showCapture ? 'rotate-180' : '')
						}}
					>
						Test trigger
					</Button>
				{/if}

				{#if !noSave}
					{@const disabled = newItem || cloudDisabled}
					<Popover notClickable>
						<Button
							size="xs2"
							{disabled}
							startIcon={{ icon: Save }}
							on:click={() => {
								dispatch('saveTrigger', {
									config: args
								})
							}}
						>
							Save
						</Button>
						<svelte:fragment slot="text">
							{#if disabled}
								{#if newItem}
									Deploy the runnable to enable trigger creation
								{:else if cloudDisabled}
									{capitalize(triggerType)} triggers are disabled in the multi-tenant cloud
								{/if}
							{:else}
								Create new {captureTypeLabels[triggerType].toLowerCase()}
							{/if}
						</svelte:fragment>
					</Popover>
				{/if}
			</div>
		{/if}
	</svelte:fragment>

	{#if isEditor}
		<CaptureWrapper
			{path}
			{isFlow}
			captureType={triggerType}
			{hasPreprocessor}
			{canHavePreprocessor}
			on:applyArgs
			on:addPreprocessor
			on:updateSchema
			on:saveTrigger
			on:testWithArgs
			bind:args
			{data}
			{showCapture}
		/>
	{:else}
		<TriggersWrapper {path} {isFlow} {triggerType} {cloudDisabled} {args} {data} />
	{/if}
</Section>
