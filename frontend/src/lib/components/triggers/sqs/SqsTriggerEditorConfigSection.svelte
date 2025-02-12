<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import QueueSetup from './QueueSetup.svelte'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'

	let urlError: string = ''
	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let queue_url = ''
	export let aws_resource_path = ''
	export let message_attributes: string[] = []

	$: isValid =
		urlError === '' && !emptyStringTrimmed(aws_resource_path) && !emptyStringTrimmed(queue_url)
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			captureType="sqs"
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
	<Section label="SQS" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Section small label="AWS connection setup">
				<div class="flex flex-col gap-1">
					<p class="text-xs mb-1 text-tertiary">
						Select an AWS resource with credentials to authenticate your account. <Required
							required={true}
						/>
					</p>
					<ResourcePicker resourceType="aws" bind:value={aws_resource_path} />
					{#if isValid}
						<TestTriggerConnection kind="sqs" args={{ aws_resource_path, queue_url }} />
					{/if}
				</div>
			</Section>
			<QueueSetup bind:can_write bind:queue_url bind:message_attributes />
		</div>
	</Section>
</div>
