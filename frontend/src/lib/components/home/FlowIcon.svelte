<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { twMerge } from 'tailwind-merge'

	export let color: string = 'black'
</script>

<div class={twMerge(`text-${color}`, $$props.class)}>
	<BarsStaggered size={14} />
</div>
