<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'

	export let content: string
	export let title: string | undefined = undefined
</script>

{#if title !== undefined}
	<div class="text-xs font-semibold">{title}</div>
{/if}

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="grow min-w-0 w-full px-2 py-1 border flex items-center cursor-pointer bg-surface-secondary text-primary justify-between rounded-md"
	on:click={(e) => {
		e.preventDefault()
		copyToClipboard(content)
	}}
>
	<div class="text-xs truncate whitespace-no-wrap grow">{content}</div>
	<Clipboard size={12} class="flex-shrink-0" />
</div>
