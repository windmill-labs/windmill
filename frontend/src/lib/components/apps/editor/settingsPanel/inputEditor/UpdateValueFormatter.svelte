<script lang="ts">
	import { tableMetadataShared } from '$lib/components/apps/components/display/dbtable/AppDbExplorer.svelte'

	type Column = {
		field: string
		valueFormatter: string
	}

	export let value: Column | undefined

	let synced: boolean = false

	$: if (
		tableMetadataShared?.find((x) => x.columnname === value?.field)?.datatype === 'jsonb' &&
		value &&
		(value?.valueFormatter === undefined || value?.valueFormatter === null) &&
		!synced
	) {
		synced = true
		value.valueFormatter = 'JSON.stringify(value, null, 2)'
	}
</script>
