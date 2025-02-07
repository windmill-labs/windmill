<script lang="ts">
	import MultiSelect from 'svelte-multiselect'
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import { X } from 'lucide-svelte'

	export let message_attributes: string[]
	export let queue_url = ''
	export let can_write: boolean = false

	let dirtyUrl: boolean = false
	let urlError: string = ''
	let validateTimeout: NodeJS.Timeout | undefined = undefined

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
</script>

<Section label="Queue url">
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
<Section label="Message attributes">
	<div class="flex flex-col gap-1">
		<p class="text-xs mb-1 text-tertiary">
			Select up to 10 message attribute names that you want to receive with this message. If these
			attributes are present in incoming SQS messages, their values will be included in the trigger
			data.
		</p>
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
		>
			<svelte:fragment slot="remove-icon">
				<div class="hover:text-primary p-0.5">
					<X size={12} />
				</div>
			</svelte:fragment>
		</MultiSelect>
	</div>
</Section>
