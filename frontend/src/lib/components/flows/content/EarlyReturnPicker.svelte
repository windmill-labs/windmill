<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import Select from '$lib/components/select/Select.svelte'
	import { earlyReturnCandidates } from '../earlyReturnCandidates'

	interface Props {
		modules: FlowModule[]
		value: string | undefined
	}

	let { modules, value = $bindable() }: Props = $props()

	let candidates = $derived(earlyReturnCandidates(modules))
	let selected = $derived(candidates.find((c) => c.id === value))

	// Select filters on `label` alone, so it carries the path too and a search matches either; the
	// visible id is drawn by `startSnippet`.
	let items = $derived(
		candidates.map((c) => ({
			value: c.id,
			label: c.location ? `${c.id} ${c.location}` : c.id,
			subtitle: c.location
		}))
	)
</script>

<div class="flex flex-col gap-1 max-w-sm">
	<Select
		{items}
		bind:value
		transformInputSelectedText={(_, id) => String(id)}
		itemLabelWrapperClasses="hidden"
		placeholder="Node's id"
		size="sm"
	>
		{#snippet startSnippet({ item })}
			<div>{item.value}</div>
		{/snippet}
	</Select>
	{#if selected && !selected.deterministic}
		<span class="text-2xs text-secondary">
			This node only runs if its branch is taken. Otherwise, sync calls return the flow's final
			result.
		</span>
	{/if}
</div>
