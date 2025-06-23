<script lang="ts">
	import { Splitpanes } from 'svelte-splitpanes'
	import { getSplitPanesLayout } from './SplitPanesLayout.svelte'
	import { onDestroy, onMount, setContext, tick, type Snippet } from 'svelte'
	import type { ComponentProps } from 'svelte'
	import type { Pane, SplitPanesContext } from './types'

	type SplitpanesProps = ComponentProps<Splitpanes>

	type Props = SplitpanesProps & {
		defaultSizes: number[]
		id: string
		children?: Snippet
	}

	const splitPanesLayout = getSplitPanesLayout()

	let { id, children, defaultSizes, ...rest }: Props = $props()

	let ready = $state(false)
	let isDestroying = $state(false)
	let initialPanes: Pane[] = $state(
		splitPanesLayout?.layout[id] ??
			defaultSizes.map((size) => ({
				size,
				active: false
			}))
	)
	const sizes = $derived(splitPanesLayout?.layout[id]?.map((pane) => pane.size) ?? [])

	setContext<SplitPanesContext>('splitPanesContext', {
		sizes: (index: number) => sizes[index],
		setActivePane: (index: number) => {
			if (!ready) {
				initialPanes[index].active = true
				return
			}
			splitPanesLayout?.setActivePane(id, index)
		},
		removeActivePane: async (index: number) => {
			await tick()
			if (!ready || isDestroying) {
				initialPanes[index].active = false
				return
			}
			splitPanesLayout?.removePane(id, index)
		}
	})

	function initPanes() {
		splitPanesLayout?.setPanes(id, initialPanes)
	}

	onDestroy(() => {
		isDestroying = true
		ready = false
	})

	onMount(() => {
		initPanes()
		ready = true
	})
</script>

<Splitpanes
	on:resized={({ detail }) => {
		splitPanesLayout?.handleResize(
			id,
			detail.map((d, index) => ({ size: d.size, index }))
		)
	}}
	{...rest}
>
	{@render children?.()}
</Splitpanes>
