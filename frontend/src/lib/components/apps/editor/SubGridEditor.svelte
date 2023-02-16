<script lang="ts">
	import type { GridItem } from '../types'
	import Test from './Test.svelte'

	export let subGrid: GridItem[]
</script>

<Test bind:lazyGrid={subGrid} />
