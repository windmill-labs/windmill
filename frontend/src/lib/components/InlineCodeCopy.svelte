<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'
	export let content: string
</script>

<pre
	class="bg-surface-secondary text-secondary p-2 font-mono text-sm whitespace-pre-wrap my-2 rounded-md">
	{content} 
	<button on:click={() => copyToClipboard(content)} class="cursor-pointer ml-2">
		<Clipboard />
	</button>
</pre>
