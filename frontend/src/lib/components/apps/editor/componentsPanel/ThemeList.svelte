<script lang="ts">
	import TableSimple from '$lib/components/TableSimple.svelte'
	import { listThemes, type Theme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'

	let themes: Theme[] = []

	async function getThemes() {
		themes = await listThemes($workspaceStore!)
	}

	onMount(() => {
		getThemes()
	})
</script>

<div class="p-2">
	{#if Array.isArray(themes) && themes.length > 0}
		<TableSimple
			size="sm"
			headers={['Name', 'Path']}
			data={themes}
			keys={['name', 'value', 'description']}
		/>
	{/if}
</div>
