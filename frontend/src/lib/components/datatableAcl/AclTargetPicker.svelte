<script lang="ts">
	import Select from '../select/Select.svelte'

	let {
		schemas,
		tables,
		schemasLoading = false,
		schema = $bindable(),
		table = $bindable()
	}: {
		/** The database's schemas, as the editor last read them. */
		schemas: string[]
		/** The picked schema's tables, as the editor last read them. */
		tables: string[]
		/** The editor has not read the database yet, so `schemas` is not known to be empty. */
		schemasLoading?: boolean
		/** Unset for the database itself. */
		schema?: string
		/** Unset for the whole schema. */
		table?: string
	} = $props()
</script>

<div class="flex flex-wrap items-center gap-2">
	<Select
		items={schemas.map((s) => ({ value: s, label: s }))}
		bind:value={
			() => schema,
			(s) => {
				schema = s
				table = undefined
			}
		}
		placeholder="The database itself"
		clearable
		loading={schemasLoading}
		size="sm"
		class="w-56"
	/>
	{#if schema}
		<Select
			items={tables.map((t) => ({ value: t, label: t }))}
			bind:value={table}
			placeholder="The whole schema"
			clearable
			size="sm"
			class="w-56"
		/>
	{/if}
</div>
