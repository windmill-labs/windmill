<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { twMerge } from 'tailwind-merge'


	interface Props {
		color?: string;
		class?: string;
	}

	let { color = 'black', class: className = '' }: Props = $props();
	
</script>

<div class={twMerge(`text-${color}`, className)}>
	<BarsStaggered size={14} />
</div>
