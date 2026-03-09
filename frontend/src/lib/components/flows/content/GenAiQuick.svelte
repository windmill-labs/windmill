<script lang="ts">
	import { createBubbler } from 'svelte/legacy';

	const bubble = createBubbler();
	import { Wand2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		funcDesc: string;
		selected: boolean;
		lang: string;
	}

	let { funcDesc, selected, lang }: Props = $props();
	const dispatch = createEventDispatcher()

	const onKeyDown = (e: KeyboardEvent) => {
		if (selected && e.key === 'Enter') {
			e.preventDefault()
			dispatch('click')
		}
	}
</script>

<svelte:window onkeydown={onKeyDown} />
<button
	class="px-3 py-2 gap-2 w-full text-left hover:bg-surface-hover flex flex-row items-center transition-all rounded-md {selected
		? 'bg-surface-hover'
		: ''}"
	onclick={bubble('click')}
>
	<Wand2 size={14} class="text-ai" />

	<span class="grow truncate text-left text-2xs text-primary font-normal">
		Generate "{funcDesc}" in {lang}
	</span>
	{#if selected}
		<kbd class="!text-xs">&crarr;</kbd>
	{/if}
</button>
