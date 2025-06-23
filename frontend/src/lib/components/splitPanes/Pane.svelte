<script lang="ts">
	import { Pane } from 'svelte-splitpanes'
	import type { Snippet } from 'svelte'
	import { getContext, onDestroy, onMount, type ComponentProps } from 'svelte'
	import type { SplitPanesContext } from './types'

	type SplitpanesProps = ComponentProps<Pane>

	export type Props = SplitpanesProps & {
		index: number
		children?: Snippet
	}

	let { index, children, ...rest }: Props = $props()

	const { sizes, setActivePane, removeActivePane } =
		getContext<SplitPanesContext>('splitPanesContext') ?? {}

	onMount(() => {
		setActivePane(index)
	})

	onDestroy(() => {
		removeActivePane(index)
	})
</script>

<Pane size={sizes?.(index)} {...rest}>
	{@render children?.()}
</Pane>
