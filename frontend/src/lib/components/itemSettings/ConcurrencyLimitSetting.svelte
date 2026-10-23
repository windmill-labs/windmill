<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { SecondsInput } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		limit: number | undefined
		timeWindowS: number | undefined
		concurrencyKey: string | undefined
		keyPlaceholder: string
	}

	let {
		kind,
		limit = $bindable(),
		timeWindowS = $bindable(),
		concurrencyKey = $bindable(),
		keyPlaceholder
	}: Props = $props()

	let disabled = $derived(!$enterpriseLicense)
</script>

<SettingToggle
	setting="concurrency"
	label="Concurrency limit"
	tooltip={`Caps how many runs of this ${kind} can start within a time window. The runs over the limit wait in the queue.`}
	documentationLink={kind === 'flow'
		? 'https://www.windmill.dev/docs/flows/concurrency_limit'
		: 'https://www.windmill.dev/docs/core_concepts/concurrency_limits'}
	eeOnly
	{disabled}
	checked={limit !== undefined && limit > 0}
	onchange={(on) => {
		if (on) {
			limit = 1
		} else {
			limit = undefined
			timeWindowS = undefined
			concurrencyKey = undefined
		}
	}}
>
	<Label label="Max number of executions within the time window">
		<TextInput class="w-24" bind:value={limit} inputProps={{ type: 'number', min: 1, disabled }} />
	</Label>
	<Label label="Time window in seconds">
		<SecondsInput {disabled} bind:seconds={timeWindowS} />
	</Label>
	<Label label="Custom concurrency key (optional)">
		{#snippet header()}
			<Tooltip
				documentationLink="https://www.windmill.dev/docs/core_concepts/concurrency_limits#custom-concurrency-key"
			>
				Concurrency keys are global. Use `$workspace` to make one workspace specific, and
				`$args[name_of_arg]` to use an argument's value.
			</Tooltip>
		{/snippet}
		<TextInput
			bind:value={concurrencyKey}
			inputProps={{ type: 'text', disabled, placeholder: keyPlaceholder }}
		/>
	</Label>
</SettingToggle>
