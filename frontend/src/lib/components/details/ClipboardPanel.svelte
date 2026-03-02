<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Copy } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { inputSizeClasses } from '../text_input/TextInput.svelte'


	interface Props {
		content: string;
		title?: string | undefined;
		size?: 'sm' | 'md';
		disabled?: boolean;
		class?: string;
	}

	let {
		content,
		title = undefined,
		size = 'md',
		disabled = false,
		class: className = ''
	}: Props = $props();
	
</script>

{#if title !== undefined}
	<div class="text-xs font-semibold">{title}</div>
{/if}

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge(
		'grow min-w-0 w-full border flex items-center bg-surface-secondary hover:opacity-60 transition-opacity text-primary justify-between rounded-md',
		inputSizeClasses[size],
		className
	)}
	class:cursor-not-allowed={disabled}
	class:cursor-pointer={!disabled}
	onclick={(e) => {
		if (disabled) {
			return
		}
		e.preventDefault()
		copyToClipboard(content)
	}}
>
	<div class={twMerge('truncate whitespace-no-wrap grow text-xs')}>{content}</div>
	<Copy size={12} class="flex-shrink-0" />
</div>
