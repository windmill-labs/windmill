<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let content: string
	export let title: string | undefined = undefined
	export let size: 'sm' | 'md' = 'sm'
	export let disabled = false
</script>

{#if title !== undefined}
	<div class="text-xs font-semibold">{title}</div>
{/if}

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="grow min-w-0 w-full px-2 py-1 border flex items-center bg-surface-secondary text-primary justify-between rounded-md"
	class:cursor-not-allowed={disabled}
	class:cursor-pointer={!disabled}
	on:click={(e) => {
		if (disabled) {
			return
		}
		e.preventDefault()
		copyToClipboard(content)
	}}
>
	<div class={twMerge('truncate whitespace-no-wrap grow', size === 'sm' ? 'text-xs' : 'text-sm')}
		>{content}</div
	>
	<Clipboard size={12} class="flex-shrink-0" />
</div>
