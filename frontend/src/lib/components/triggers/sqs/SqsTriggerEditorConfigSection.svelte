<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed } from '$lib/utils'

	let urlError: string = ''
	let validateTimeout: NodeJS.Timeout | undefined = undefined

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let queue_url = ''
	export let aws_resource_path = ''
	export let dirtyUrl: boolean = false

	function validateUrl(queue_url: string | undefined) {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(() => {
			if (!queue_url || /^(https:)\/\/[^\s]+$/.test(queue_url) === false) {
				urlError = 'Queue url must start with https://'
			} else {
				urlError = ''
			}
			validateTimeout = undefined
		}, 500)
	}
	$: validateUrl(queue_url)
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
	<Section label="Amazon SQS" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Section small label="AWS connection setup">
				<p class="text-xs mb-1 text-tertiary">
					Select an AWS resource with credentials to authenticate your account. <Required
						required={true}
					/>
				</p>
				<ResourcePicker resourceType="aws" bind:value={aws_resource_path} />
			</Section>
			<Section small label="SQS Queue Selection">
				<div class="flex flex-col gap-1">
					<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
						<div class="flex flex-col gap-1">
							<p class="text-xs text-tertiary">
								Provide the URL of the SQS queue the application should listen to. <Required
									required={true}
								/>
							</p>
						</div>
					</div>
					<input
						type="text"
						autocomplete="off"
						bind:value={queue_url}
						disabled={!can_write}
						on:input={() => {
							dirtyUrl = true
						}}
						placeholder="https://example.com"
						class={urlError === ''
							? ''
							: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
					/>
					<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
						{dirtyUrl ? urlError : ''}
					</div>
				</div>
			</Section>
		</div>
	</Section>
</div>
