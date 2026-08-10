<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import { Plus, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import TriggerFilterList from './TriggerFilterList.svelte'
	import {
		groupItems,
		groupLogic,
		isFilterGroup,
		makeGroup,
		type FilterLogic,
		type FilterNode
	} from './filters'

	interface Props {
		filters: FilterNode[]
		logic: FilterLogic
		disabled?: boolean
		/** Nesting level, 0 at the top. */
		depth?: number
	}

	let { filters = $bindable(), logic = $bindable(), disabled = false, depth = 0 }: Props = $props()

	// Deeper nesting is supported by the backend but stops being readable in this editor.
	const MAX_DEPTH = 3

	const logicItems = [
		{ label: 'all criteria (AND)', value: 'and' as const },
		{ label: 'any criterion (OR)', value: 'or' as const }
	]

	function add(node: FilterNode) {
		filters = [...filters, node]
	}
</script>

<div class="flex flex-col gap-2">
	{#if depth > 0 || filters.length > 0}
		<div class="max-w-xs">
			<Select items={logicItems} bind:value={logic} {disabled} size="sm" />
		</div>
	{/if}

	<!-- Keyed by node, not index: the value editor seeds itself from `code` once, so reusing
	     a row for a different filter after a deletion would leave the old value on screen. -->
	{#each filters as filter, i (filter)}
		<div class="flex w-full gap-2 items-start">
			{#if isFilterGroup(filter)}
				<div class="w-full border p-2 rounded-md bg-surface-secondary">
					<TriggerFilterList
						bind:filters={
							() => groupItems(filter),
							(items) => (filters[i] = makeGroup(groupLogic(filter), items))
						}
						bind:logic={
							() => groupLogic(filter),
							(nested) => (filters[i] = makeGroup(nested, groupItems(filter)))
						}
						{disabled}
						depth={depth + 1}
					/>
				</div>
			{:else}
				<div class="w-full flex flex-col gap-2 border p-2 rounded-md bg-surface">
					<label class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Key</div>
						<TextInput bind:value={filter.key} inputProps={{ disabled }} />
					</label>
					<div class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Value</div>
						<JsonEditor bind:value={filter.value} code={JSON.stringify(filter.value)} {disabled} />
					</div>
					{#if filter.key}
						{@const isObject = filter.value !== null && typeof filter.value === 'object'}
						<div class="text-xs text-tertiary font-mono mt-2 p-2 bg-surface-secondary rounded">
							payload.{filter.key}
							{isObject ? '⊇' : '=='}
							{JSON.stringify(filter.value)}
						</div>
					{/if}
				</div>
			{/if}
			<button
				transition:fade|local={{ duration: 100 }}
				class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover mt-1"
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

	<div class="flex items-baseline gap-2">
		<Button
			variant="default"
			size="xs"
			btnClasses="mt-1"
			onclick={() => add({ key: '', value: '' })}
			{disabled}
			startIcon={{ icon: Plus }}
		>
			Add filter
		</Button>
		{#if depth < MAX_DEPTH}
			<Button
				variant="default"
				size="xs"
				btnClasses="mt-1"
				onclick={() => add(makeGroup(logic === 'or' ? 'and' : 'or', []))}
				{disabled}
				startIcon={{ icon: Plus }}
			>
				Add group
			</Button>
		{/if}
	</div>
</div>
