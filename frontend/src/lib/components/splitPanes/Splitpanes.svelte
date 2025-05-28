<script lang="ts">
	import { Splitpanes } from 'svelte-splitpanes'
	import { getSplitPanesLayout } from './SplitPanesLayout.svelte'
	import { onDestroy, setContext, type Snippet } from 'svelte'
	import type { ComponentProps } from 'svelte'
	import type { Pane, SplitPanesContext } from './types'

	type SplitpanesProps = ComponentProps<Splitpanes>

	type Props = SplitpanesProps & {
		defaultSize: number[]
		id: string
		children?: Snippet
	}

	const splitPanesLayout = getSplitPanesLayout()

	let { id, children, defaultSize, ...rest }: Props = $props()
	const sizes = $derived(splitPanesLayout?.layout[id]?.map((pane) => pane.size ?? 0) ?? [])

	setContext<SplitPanesContext>('splitPanesContext', {
		sizes: (index: number) => sizes[index],
		setActivePane: (index: number) => {
			panes[index].active = true
		}
	})

	function initPane(defaultSize: number[]): Pane[] {
		// Initialize all panes with default sizes
		const panes = defaultSize.map((size, idx) => ({ size, active: false }))
		return panes
	}

	let panes: Pane[] = $state(initPane(defaultSize))

	onDestroy(() => {
		splitPanesLayout?.handleSplitPaneDestroy(id)
	})
</script>

<Splitpanes
	on:resized={({ detail }) => {
		splitPanesLayout?.handleResize(
			id,
			detail.map((d, index) => ({ size: d.size, index }))
		)
	}}
	on:pane-add={({ detail }) => {
		splitPanesLayout?.addPane(id, detail.index)
	}}
	on:pane-remove={({ detail }) => {
		splitPanesLayout?.removePane(id, detail.removed.index)
	}}
	on:ready={() => {
		splitPanesLayout?.setPanes(id, panes)
	}}
	{...rest}
>
	{@render children?.()}
</Splitpanes>
