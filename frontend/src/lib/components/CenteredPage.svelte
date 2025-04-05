<script>
	import { twMerge } from 'tailwind-merge'
	import Scroll from '$lib/components/Scroll.svelte'
	import { layoutStore } from '$lib/layout-context'

	export let useScroll = false
</script>

{#if useScroll}
	<Scroll style={`height: calc(100vh - ${$layoutStore.topBarHeight}px)`}>
		<div class="pb-8">
			<div class={twMerge('max-w-7xl mx-auto px-4 sm:px-6 md:px-8', $$restProps.class)}>
				<slot />
			</div>
		</div>
	</Scroll>
{:else}
	<div class="pb-8">
		<div class={twMerge('max-w-7xl mx-auto px-4 sm:px-6 md:px-8', $$restProps.class)}>
			<slot />
		</div>
	</div>
{/if}
