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
	import { Plus, X } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ArgInput from '$lib/components/ArgInput.svelte'
	import ItemPicker from '$lib/components/ItemPicker.svelte'
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import { Button } from '$lib/components/common'
	import { VariableService, type AwsAuthResourceType } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let queue_url = ''
	export let aws_resource_path = ''
	export let aws_auth_resource_type: AwsAuthResourceType = 'credentials'
	export let message_attributes: string[] = []

	async function loadVariables() {
		return await VariableService.listVariable({ workspace: $workspaceStore ?? '' })
	}
	let itemPicker: ItemPicker
	let variableEditor: VariableEditor
	let cached: string[] = []
	let all_attributes = message_attributes.includes('All')
	let tab: 'specific' | 'all' = all_attributes ? 'all' : 'specific'

	$: isValid = !emptyStringTrimmed(aws_resource_path) && !emptyStringTrimmed(queue_url)
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
					<div class="flex flex-col gap-3">
						<p class="text-xs text-tertiary">
							Select an AWS resource to authenticate your account. <Required required={true} />
						</p>

						<ToggleButtonGroup bind:selected={aws_auth_resource_type} on:selected={() => {
							aws_resource_path = ''
						}} let:item>
							<ToggleButton label="Credentials" value="credentials" {item} />
							<ToggleButton label="Oidc" value="oidc" {item} />
						</ToggleButtonGroup>

						{#if aws_auth_resource_type === 'credentials'}
							<ResourcePicker resourceType="aws" bind:value={aws_resource_path} />
						{:else if aws_auth_resource_type === 'oidc'}
							<ResourcePicker resourceType="aws_oidc" bind:value={aws_resource_path} />
						{/if}
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

						<ArgInput
							placeholder="https://example.com"
							resourceTypes={undefined}
							noDefaultOnSelectFirst
							{itemPicker}
							bind:value={queue_url}
							type="string"
							displayHeader={false}
							disabled={!can_write}
							{variableEditor}
							compact
							noMargin
						/>
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
						ulOptionsClass={'!bg-surface !text-sm'}
						ulSelectedClass="!text-sm"
						outerDivClass="!bg-surface !min-h-[38px] !border-[#d1d5db]"
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

<ItemPicker
	bind:this={itemPicker}
	pickCallback={(path, _) => {
		queue_url = '$var:' + path
	}}
	tooltip="Variables are dynamic values that have a key associated to them and can be retrieved during the execution of a Script or Flow."
	documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
	itemName="Variable"
	extraField="path"
	loadItems={loadVariables}
	buttons={{ 'Edit/View': (x) => variableEditor.editVariable(x) }}
>
	<div slot="submission" class="flex flex-row">
		<Button
			variant="border"
			color="blue"
			size="sm"
			startIcon={{ icon: Plus }}
			on:click={() => {
				variableEditor.initNew()
			}}
		>
			New variable
		</Button>
	</div>
</ItemPicker>

<VariableEditor bind:this={variableEditor} on:create={itemPicker.openDrawer} />
