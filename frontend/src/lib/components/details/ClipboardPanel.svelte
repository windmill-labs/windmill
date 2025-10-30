<!-- @migration-task Error while migrating Svelte code: $$props is used together with named props in a way that cannot be automatically migrated. -->
<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { inputSizeClasses } from '../text_input/TextInput.svelte'

	export let content: string
	export let title: string | undefined = undefined
	export let size: 'sm' | 'md' = 'md'
	export let disabled = false
</script>

{#if title !== undefined}
	<div class="text-xs font-semibold">{title}</div>
{/if}

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={twMerge(
		'grow min-w-0 w-full border flex items-center bg-surface-secondary hover:opacity-60 transition-opacity text-primary justify-between rounded-md',
		inputSizeClasses[size],
		$$props.class
	)}
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
	<div class={twMerge('truncate whitespace-no-wrap grow text-xs')}>{content}</div>
	<Clipboard size={12} class="flex-shrink-0" />
</div>
