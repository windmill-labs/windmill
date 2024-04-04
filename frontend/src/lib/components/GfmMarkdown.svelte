<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'

	export let md: string
</script>

<div class="!prose-xs pgap">
	<Markdown {md} plugins={[gfmPlugin()]} />
</div>

<style global>
	.pgap > p {
		margin-top: 0.5rem;
		margin-bottom: 0.5rem;
	}
</style>
