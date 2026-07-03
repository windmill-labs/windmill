<script lang="ts">
	import { Button } from '$lib/components/common'
	import Section from '$lib/components/Section.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { Plus, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import JsonEditor from '$lib/components/JsonEditor.svelte'

	interface Props {
		filters: { key: string; value: any }[]
		filterLogic: 'and' | 'or'
		disabled?: boolean
		// Set when the runnable receives the payload base64-encoded (e.g. Kafka).
		// Filters always run on the message parsed as JSON, so we clarify the distinction.
		payloadBase64Encoded?: boolean
	}

	let {
		filters = $bindable([]),
		filterLogic = $bindable(),
		disabled = false,
		payloadBase64Encoded = false
	}: Props = $props()

	const filterLogicItems = [
		{ label: 'all criteria (AND)', value: 'and' as const },
		{ label: 'any criterion (OR)', value: 'or' as const }
	]

	let description = $derived(
		filterLogic === 'or'
			? 'Filters will limit the execution of the trigger to only messages that match any criterion.'
			: 'Filters will limit the execution of the trigger to only messages that match all criteria.'
	)

	let filterHelp = $derived(
		'The JSON filter checks if the value at the key is equal or a superset of the filter value. ' +
			'Keys match top-level fields of the message (parsed as JSON); to match a nested field, set an object value (e.g. key data, value {"status": "active"}).' +
			(payloadBase64Encoded
				? ' The runnable still receives the payload base64-encoded; filters run on the message before that encoding.'
				: '')
	)
</script>

<Section label="Filters">
	<p class="text-xs mb-1 text-primary">
		{description}<br />
		{filterHelp}
	</p>
	{#if filters.length > 0}
		<div class="mt-2 mb-1 max-w-xs">
			<Select items={filterLogicItems} bind:value={filterLogic} {disabled} size="sm" />
		</div>
	{/if}
	<div class="flex flex-col gap-4 mt-1">
		{#each filters as v, i (i)}
			<div class="flex w-full gap-2 items-center">
				<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
					<label class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Key</div>
						<input type="text" bind:value={v.key} {disabled} />
					</label>
					<div class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Value</div>
						<JsonEditor bind:value={v.value} code={JSON.stringify(v.value)} {disabled} />
					</div>
					{#if v.key}
						{@const isObject = v.value !== null && typeof v.value === 'object'}
						<div class="text-xs text-tertiary font-mono mt-2 p-2 bg-surface-secondary rounded">
							payload.{v.key}
							{isObject ? '⊇' : '=='}
							{JSON.stringify(v.value)}
						</div>
					{/if}
				</div>
				<button
					transition:fade|local={{ duration: 100 }}
					class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
					aria-label="Clear"
					onclick={() => {
						filters = filters.filter((_, index) => index !== i)
					}}
					{disabled}
				>
					<X size={14} />
				</button>
			</div>
		{/each}

		<div class="flex items-baseline">
			<Button
				variant="default"
				size="xs"
				btnClasses="mt-1"
				onclick={() => {
					if (filters == undefined || !Array.isArray(filters)) {
						filters = []
					}
					filters = filters.concat({
						key: '',
						value: ''
					})
				}}
				{disabled}
				startIcon={{ icon: Plus }}
			>
				Add filter
			</Button>
		</div>
	</div>
</Section>
