<script lang="ts">
	import { Splitpanes } from 'svelte-splitpanes'
	import { getSplitPanesLayout } from './SplitPanesLayout.svelte'
	import { onDestroy, onMount, setContext, type Snippet } from 'svelte'
	import type { ComponentProps } from 'svelte'

	type SplitpanesProps = ComponentProps<Splitpanes>

	type Props = SplitpanesProps & {
		id: string
		children?: Snippet
	}

	let { id, children, ...rest }: Props = $props()
	let timeout: ReturnType<typeof setTimeout> | undefined = undefined

	setContext<string>('splitPanesId', id)

	const splitPanesLayout = getSplitPanesLayout()

	onMount(() => {
		splitPanesLayout?.handleSplitPaneReady(id)
	})

	onDestroy(() => {
		splitPanesLayout?.handleSplitPaneDestroy(id)
		clearTimeout(timeout)
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
