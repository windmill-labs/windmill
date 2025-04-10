<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import MultiSelect from 'svelte-multiselect'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import { X } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let queue_url = ''
	export let aws_resource_path = ''
	export let message_attributes: string[] = []
	let cached: string[] = []
	let dirtyUrl: boolean = false
	let urlError: string = ''
	let validateTimeout: NodeJS.Timeout | undefined = undefined
	let all_attributes = message_attributes.includes('All')
	let tab: 'specific' | 'all' = all_attributes ? 'all' : 'specific'
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
	<Section label="SQS" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Subsection label="Connection setup">
				<div class="flex flex-col gap-3">
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
				</div>
			</Subsection>
			<Subsection
				tooltip="  When 'All Attributes' is selected, all message attributes from the received message are included with the message. When 'Specific Attributes' is selected, only the specified attributes (up to a maximum of 10) are included if they are present in the message."
				label="Message attributes"
			>
				<div class="mt-2">
					<ToggleButtonGroup
						selected={tab}
						on:selected={({ detail }) => {
							if (detail === 'all') {
								cached = message_attributes
								message_attributes = ['All']
							} else {
								message_attributes = cached
							}
							tab = detail
						}}
						let:item
					>
						<ToggleButton value="all" label="All attributes" {item} />
						<ToggleButton value="specific" label="Specific attributes" {item} />
					</ToggleButtonGroup>
				</div>
				<div class="flex flex-col mt-3 gap-1">
					<MultiSelect
						options={message_attributes ?? []}
						allowUserOptions="append"
						bind:selected={message_attributes}
						ulOptionsClass={'bg-surface! text-sm!'}
						ulSelectedClass="text-sm!"
						outerDivClass="bg-surface! min-h-[38px]! border-[#d1d5db]!"
						noMatchingOptionsMsg=""
						createOptionMsg={null}
						duplicates={false}
						placeholder="Set message attributes"
						--sms-options-margin="4px"
						disabled={tab === 'all'}
					>
						<svelte:fragment slot="remove-icon">
							<div class="hover:text-primary p-0.5">
								<X size={12} />
							</div>
						</svelte:fragment>
					</MultiSelect>
				</div>
			</Subsection>
		</div>
	</Section>
</div>
