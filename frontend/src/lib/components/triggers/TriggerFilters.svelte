<script lang="ts">
	import { Button } from '$lib/components/common'
	import Section from '$lib/components/Section.svelte'
	import { Plus, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import JsonEditor from '$lib/components/JsonEditor.svelte'

	interface Props {
		filters: { key: string; value: any }[]
		disabled?: boolean
	}

	let { filters = $bindable([]), disabled = false }: Props = $props()
</script>

<Section label="Filters">
	<p class="text-xs mb-1 text-primary">
		Filters will limit the execution of the trigger to only messages that match all criteria.<br />
		The JSON filter checks if the value at the key is equal or a superset of the filter value.
	</p>
	<div class="flex flex-col gap-4 mt-1">
		{#each filters as v, i (i)}
			<div class="flex w-full gap-2 items-center">
				<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
					<label class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Key</div>
						<input type="text" bind:value={v.key} {disabled} />
					</label>
					<!-- svelte-ignore a11y_label_has_associated_control -->
					<label class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Value</div>
						<JsonEditor bind:value={v.value} code={JSON.stringify(v.value)} {disabled} />
					</label>
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
