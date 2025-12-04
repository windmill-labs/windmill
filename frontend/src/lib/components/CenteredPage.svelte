<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		class?: string
		children?: import('svelte').Snippet<[{ width: number }]>
	}

	let { class: clazz = '', children }: Props = $props()

	let width = $state(0)
</script>

<div class="pb-8">
	<div class={twMerge('max-w-7xl mx-auto px-4 sm:px-6 md:px-8', clazz)} bind:clientWidth={width}
		>{@render children?.({ width })}</div
	>
</div>
