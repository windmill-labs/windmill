<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { twMerge } from 'tailwind-merge'

	export let color: string = 'black'

	let className: string = ''
	export { className as class }
</script>

<div class={twMerge(`text-${color}`, className)}>
	<BarsStaggered size={14} />
</div>
