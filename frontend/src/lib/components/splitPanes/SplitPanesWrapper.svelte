<script lang="ts">
	import type { Snippet } from 'svelte'

	/**
	 * This component should be used instead of `Splitpanes` if the wrapper `Splitpanes`
	 * has elements that are **NOT** `Pane` above the place of this component.
	 */

	interface Props {
		/** This element will act as the reference point to the `Splitpanes`
		 * and the top difference will be calculated from it. */
		refElement?: HTMLElement | undefined
		class?: string
		children?: Snippet
	}

	let {
		refElement = undefined,
		class: className = '',
		children
	}: Props = $props()

	let wrapper: HTMLDivElement
	let gap = $state(0)

	function getTopDifference(): number {
		const parent = refElement || wrapper.parentElement
		if (!(wrapper && parent)) return 0

		const wrapperTop = wrapper.getBoundingClientRect().top
		const parentTop = parent.getBoundingClientRect().top
		return wrapperTop - parentTop
	}

	$effect(() => {
		gap = getTopDifference()
	})
</script>

<div
	bind:this={wrapper}
	class="h-full {className}"
	style="max-height: calc(100% - {gap}px) !important;"
>
	{@render children?.()}
</div>
