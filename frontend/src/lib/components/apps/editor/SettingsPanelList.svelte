<script lang="ts">
	import type { GridItem } from '../types'

	import SettingsPanelRec from './SettingsPanelRec.svelte'

	export let subgrids: GridItem[][]

	let elems = subgrids
	$: subgrids = elems
</script>

{#if subgrids}
	{#each elems as subGrid, i (i)}
		<SettingsPanelRec bind:gridItems={subGrid} />
	{/each}
{/if}
