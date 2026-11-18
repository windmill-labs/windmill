<script lang="ts">
	// A dbt relation's columns: the real ones where the analysis pass produced
	// them — typed and in the order the model emits them — and the declared ones
	// otherwise. The description comes from `columns` either way: that is the only
	// place an author's prose lives, and a project documents a handful of forty.
	import type { DbtAssetProvenance } from './types'

	let { dbt }: { dbt: DbtAssetProvenance } = $props()

	let columns = $derived(
		dbt.column_schema?.length
			? dbt.column_schema.map((c) => ({
					name: c.name,
					type: c.type,
					description: dbt.columns?.[c.name] ?? ''
				}))
			: Object.entries(dbt.columns ?? {}).map(([name, description]) => ({
					name,
					type: undefined,
					description
				}))
	)
	let analyzed = $derived(!!dbt.column_schema?.length)
</script>

{#if columns.length > 0}
	<div class="text-2xs">
		<div class="text-tertiary mb-0.5">{analyzed ? 'columns' : 'columns declared'}</div>
		<div class="flex flex-col gap-0.5">
			{#each columns as col (col.name)}
				<div class="flex gap-2">
					<span class="font-mono text-primary shrink-0">{col.name}</span>
					{#if col.type}
						<span class="font-mono text-tertiary shrink-0">{col.type}</span>
					{/if}
					<span class="text-secondary truncate">{col.description}</span>
				</div>
			{/each}
		</div>
		<!-- `manifest.json` carries declared columns only, so without the analysis
		     pass this list is what an author wrote down rather than what the model
		     produces. -->
		{#if !analyzed}
			<div class="text-tertiary mt-0.5">
				Declared metadata. Set `column_lineage: true` in the descriptor for the real column schema,
				typed and in the order the model produces it.
			</div>
		{/if}
	</div>
{/if}
