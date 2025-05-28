<script lang="ts">
	import { Pane } from 'svelte-splitpanes'
	import { getSplitPanesLayout } from './SplitPanesLayout.svelte'
	import type { Snippet } from 'svelte'
	import { type ComponentProps, getContext, onDestroy, onMount } from 'svelte'

	type SplitpanesProps = ComponentProps<Pane>

	type Props = SplitpanesProps & {
		index: number
		defaultSize: number
		children?: Snippet
	}

	let { index, defaultSize, children, ...rest }: Props = $props()

	const splitPanesId = getContext<string>('splitPanesId')

	const splitPanesLayout = getSplitPanesLayout()

	onMount(() => {
		splitPanesLayout?.mountPane(splitPanesId, index, defaultSize)
	})

	onDestroy(() => {
		splitPanesLayout?.unmountPane(splitPanesId, index)
	})
</script>

<Pane size={splitPanesLayout?.layout[splitPanesId]?.[index]?.size ?? defaultSize} {...rest}>
	{@render children?.()}
</Pane>
