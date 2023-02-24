<script lang="ts">
	import type { GridItem } from '../../types'
	import InlineScriptEditorRec from './InlineScriptEditorRec.svelte'

	export let subgrids: GridItem[][]
	export let selectedScriptComponentId: string | undefined = undefined

	let elems = subgrids
	$: subgrids = elems
</script>

{#if subgrids}
	{#each elems as subGrid, i (i)}
		<InlineScriptEditorRec {selectedScriptComponentId} bind:gridItems={subGrid} />
	{/each}
{/if}
