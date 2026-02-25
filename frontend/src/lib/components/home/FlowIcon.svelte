<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		color?: string
		class?: string
	}

	let { color = 'black', class: c = '' }: Props = $props()
</script>

<div class={twMerge(`text-${color}`, c)}>
	<BarsStaggered size={14} />
</div>
