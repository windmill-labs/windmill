<script lang="ts">
	import {Markdown, type Plugin} from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import rehypeRaw from 'rehype-raw';

	export let md: string
	const plugins: Plugin[] = [gfmPlugin(),{ rehypePlugin: [rehypeRaw] }];
</script>

<div class="!prose-xs pgap">
	<Markdown {md} plugins={plugins} />
</div>

<style global>
	.pgap > p {
		margin-top: 0.5rem;
		margin-bottom: 0.5rem;
	}
</style>
