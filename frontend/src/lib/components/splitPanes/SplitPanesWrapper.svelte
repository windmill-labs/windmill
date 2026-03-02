<script lang="ts">
	import { afterUpdate } from 'svelte'

	/**
	 * This component should be used instead of `Splitpanes` if the wrapper `Splitpanes`
	 * has elements that are **NOT** `Pane` above the place of this component.
	 */

	/** This element will act as the reference point to the `Splitpanes`
	 * and the top difference will be calculated from it. */
	export let refElement: HTMLElement | undefined = undefined
	let wrapper: HTMLDivElement
	let gap = 0

	function getTopDifference(): number {
		const parent = refElement || wrapper.parentElement
		if (!(wrapper && parent)) return 0

		const wrapperTop = wrapper.getBoundingClientRect().top
		const parentTop = parent.getBoundingClientRect().top
		return wrapperTop - parentTop
	}

	afterUpdate(() => {
		gap = getTopDifference()
	})
</script>

<div
	bind:this={wrapper}
	class="h-full {$$props.class || ''}"
	style="max-height: calc(100% - {gap}px) !important;"
>
	<slot />
</div>
