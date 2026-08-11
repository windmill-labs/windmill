<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import { Plus, X } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import TriggerFilterList from './TriggerFilterList.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import {
		fieldMode,
		groupItems,
		groupOp,
		isFilterGroup,
		leafField,
		makeGroup,
		makeLeaf,
		type GroupOp,
		type FilterNode
	} from './filters'

	interface Props {
		filters: FilterNode[]
		logic: GroupOp
		disabled?: boolean
		/** Nesting level, 0 at the top. */
		depth?: number
	}

	let { filters = $bindable(), logic = $bindable(), disabled = false, depth = 0 }: Props = $props()

	// Deeper nesting is supported by the backend but stops being readable in this editor.
	const MAX_DEPTH = 3

	// Negation is offered on groups only: the root's operator is the trigger's
	// `filter_logic` column, which has no value for it.
	let logicItems = $derived([
		{ label: 'all criteria (AND)', value: 'and' as const },
		{ label: 'any criterion (OR)', value: 'or' as const },
		...(depth > 0 ? [{ label: 'no criterion (NONE)', value: 'none' as const }] : [])
	])

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
							() => groupItems(filter), (items) => (filters[i] = makeGroup(groupOp(filter), items))
						}
						bind:logic={
							() => groupOp(filter),
							(nested) => (filters[i] = makeGroup(nested, groupItems(filter)))
						}
						{disabled}
						depth={depth + 1}
					/>
				</div>
			{:else}
				{@const mode = fieldMode(filter)}
				{@const field = leafField(filter)}
				<div class="w-full flex flex-col gap-2 border p-2 rounded-md bg-surface">
					<div class="flex flex-col w-full">
						<div class="flex flex-row items-center mb-2">
							<ToggleButtonGroup
								selected={mode}
								{disabled}
								on:selected={(e) => (filters[i] = makeLeaf(e.detail, field, filter.value))}
							>
								{#snippet children({ item })}
									<ToggleButton
										value="key"
										label="Key"
										small
										{item}
										tooltip="A top-level field of the message"
									/>
									<ToggleButton
										value="path"
										label="Path"
										small
										{item}
										tooltip="A dotted path into nested objects, e.g. a.b.c. Does not traverse arrays."
									/>
								{/snippet}
							</ToggleButtonGroup>
						</div>
						{#if 'path' in filter}
							<TextInput bind:value={filter.path} inputProps={{ disabled, placeholder: 'a.b.c' }} />
						{:else}
							<TextInput bind:value={filter.key} inputProps={{ disabled }} />
						{/if}
					</div>
					<div class="flex flex-col w-full">
						<div class="text-secondary text-sm mb-2">Value</div>
						<JsonEditor bind:value={filter.value} code={JSON.stringify(filter.value)} {disabled} />
					</div>
					{#if field}
						{@const isObject = filter.value !== null && typeof filter.value === 'object'}
						<div class="text-xs text-tertiary font-mono mt-2 p-2 bg-surface-secondary rounded">
							payload.{field}
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
