<script lang="ts">
	import { createScrollArea, melt, type ScrollAreaType } from '@melt-ui/svelte'
	export let type: ScrollAreaType = 'hover'
	const {
		elements: { root, content, viewport, corner, scrollbarY, scrollbarX, thumbY, thumbX }
	} = createScrollArea({
		type,
		dir: 'ltr'
	})
</script>

<div use:melt={$root} class={$$props.class} style={$$props.style}>
	<div use:melt={$viewport} class="h-full w-full rounded-[inherit]">
		<div use:melt={$content}>
			<slot />
		</div>
	</div>
	<div
		use:melt={$scrollbarY}
		class="flex h-full w-2.5 touch-none select-none border-l border-l-transparent bg-gray-800/10 dark:bg-gray-900/10 p-px transition-colors"
	>
		<div
			use:melt={$thumbY}
			class="relative flex-1 rounded-full bg-gray-400 hover:bg-gray-600 dark:bg-gray-600 dark:hover:bg-gray-500"
		/>
	</div>
	<div
		use:melt={$scrollbarX}
		class="flex h-2.5 w-full touch-none select-none border-t border-t-transparent bg-gray-800/10 p-px"
	>
		<div
			use:melt={$thumbX}
			class="relative rounded-full bg-gray-400 hover:bg-gray-600 dark:bg-gray-600 dark:hover:bg-gray-500"
		/>
	</div>
	<div use:melt={$corner} />
</div>
