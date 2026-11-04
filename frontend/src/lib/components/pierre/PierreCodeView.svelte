<script lang="ts">
	import { untrack } from 'svelte'
	import { CodeView, type CodeViewItem, type CodeViewOptions } from '@pierre/diffs'
	import type { WorkerPoolManager } from '@pierre/diffs/worker'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { PIERRE_THEME } from './pierre'

	interface Props {
		items: CodeViewItem<undefined>[]
		options?: CodeViewOptions<undefined, undefined>
		workerPool?: WorkerPoolManager
		/** Fixed-height scroll container; CodeView virtualizes inside it. */
		class?: string
	}

	let { items, options, workerPool, class: className = '' }: Props = $props()

	const isDark = useIsDarkMode()
	let root: HTMLDivElement | undefined = $state()
	let viewer: CodeView | undefined = $state()

	function buildOptions(): CodeViewOptions<undefined, undefined> {
		return {
			theme: PIERRE_THEME,
			stickyHeaders: true,
			layout: { paddingTop: 8, paddingBottom: 8, gap: 8 },
			...options,
			themeType: isDark.val ? 'dark' : 'light'
		}
	}

	$effect(() => {
		if (!root) return
		const r = root
		const v = untrack(() => new CodeView(buildOptions(), workerPool))
		v.setup(r)
		viewer = v
		return () => {
			v.cleanUp()
			viewer = undefined
		}
	})

	$effect(() => {
		const opts = buildOptions()
		untrack(() => viewer?.setOptions(opts))
	})

	$effect(() => {
		viewer?.setItems(items)
	})
</script>

<div bind:this={root} class={className}></div>
