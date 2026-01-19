<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		class?: string
		children?: import('svelte').Snippet<[{ width: number }]>
		wrapperClasses?: string
		wrapperStyle?: string
	}

	let { class: clazz = '', children, wrapperClasses = '', wrapperStyle = '' }: Props = $props()

	let width = $state(0)
</script>

<div class={twMerge('pb-8', wrapperClasses)} style={wrapperStyle}>
	<div class={twMerge('max-w-7xl mx-auto px-4 sm:px-6 md:px-8', clazz)} bind:clientWidth={width}
		>{@render children?.({ width })}</div
	>
</div>
