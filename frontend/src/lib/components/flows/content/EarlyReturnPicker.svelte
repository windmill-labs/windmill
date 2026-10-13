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

	let items = $derived(candidates.map((c) => ({ value: c.id, label: c.id })))
</script>

<div class="flex flex-col gap-1 max-w-sm">
	<Select {items} bind:value placeholder="Node's id" size="sm" />
	{#if value && !selected}
		<span class="text-2xs text-red-600 dark:text-red-400">
			Step {value} is not one of the steps above. Pick another step.
		</span>
	{:else if selected && !selected.deterministic}
		<span class="text-2xs text-secondary">
			This step only runs if its branch is taken. Otherwise, sync calls return the flow's final
			result.
		</span>
	{/if}
</div>
