<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { emptySchema } from '$lib/utils'
	import type { AssetKind } from '$lib/gen'
	import type { PartitionSpec } from './parsePipelineAnnotations'
	import PartitionArgControl from './PartitionArgControl.svelte'

	interface Props {
		// The script's schema to render inputs for. SchemaForm normalizes/mutates
		// its schema in place, so we clone it into local state below.
		schema: any
		args: Record<string, any>
		isValid: boolean
		// When the script is `// partitioned`, the bare `partition` string arg is
		// pulled out of the SchemaForm and rendered by PartitionArgControl (a
		// cadence-appropriate date picker + missing/upstream hints) instead.
		partitionSpec?: PartitionSpec
		workspace?: string
		materializeTarget?: { kind: AssetKind; path: string }
		upstreamAssets?: { kind: AssetKind; path: string }[]
	}

	let {
		schema,
		args = $bindable(),
		isValid = $bindable(),
		partitionSpec,
		workspace = '',
		materializeTarget,
		upstreamAssets = []
	}: Props = $props()

	// Drop the `partition` property from the schema clone so SchemaForm doesn't
	// also render it as a bare string — PartitionArgControl owns `args.partition`.
	function stripPartition(s: any): any {
		if (!s || typeof s !== 'object') return s
		if (s.properties && 'partition' in s.properties) {
			delete s.properties.partition
		}
		if (Array.isArray(s.order)) {
			s.order = s.order.filter((k: string) => k !== 'partition')
		}
		if (Array.isArray(s.required)) {
			s.required = s.required.filter((k: string) => k !== 'partition')
		}
		return s
	}

	// Mutable clone SchemaForm can normalize without touching the parent's script.
	// Seeded ONCE per mount — the parent remounts this via `{#key <schema content>}`
	// when a local edit adds/removes args, so the clone reseeds without a prop→state
	// $effect, and an unchanged re-resolve (a WS bundle with the same schema) keeps
	// in-progress input.
	// svelte-ignore state_referenced_locally
	let formSchema = $state<any>(
		partitionSpec
			? stripPartition(structuredClone($state.snapshot(schema) ?? emptySchema()))
			: structuredClone($state.snapshot(schema) ?? emptySchema())
	)
</script>

<div class="flex flex-col gap-2">
	{#if partitionSpec}
		<PartitionArgControl
			spec={partitionSpec}
			bind:value={() => args.partition, (v) => (args.partition = v)}
			{workspace}
			{materializeTarget}
			{upstreamAssets}
		/>
	{/if}
	<SchemaForm bind:schema={formSchema} bind:args bind:isValid compact />
</div>
