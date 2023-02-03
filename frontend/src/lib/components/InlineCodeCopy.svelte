<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { faClipboard } from '@fortawesome/free-solid-svg-icons'
	import { Icon } from 'svelte-awesome'

	export let content: string
</script>

<pre class="bg-gray-700 text-gray-100 p-2  font-mono text-sm whitespace-pre-wrap"
	>{content} <span on:click={() => copyToClipboard(content)} class="cursor-pointer ml-2"
		><Icon data={faClipboard} /></span
	></pre
>
