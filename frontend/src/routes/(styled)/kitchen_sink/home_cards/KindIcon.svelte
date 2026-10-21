<script lang="ts">
	import { Bot, Code2, LayoutDashboard } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { Kind } from './model'

	interface Props {
		kind: Kind
		size?: number
		/** Grey until the enclosing `group` is hovered, then the kind's colour. */
		muted?: boolean
	}

	let { kind, size = 14, muted = false }: Props = $props()

	// RowIcon's colours, spelled out with their hover forms so Tailwind sees every class.
	const COLOUR: Record<Kind, { rest: string; hover: string }> = {
		flow: { rest: 'text-teal-500', hover: 'group-hover:text-teal-500' },
		app: { rest: 'text-orange-500', hover: 'group-hover:text-orange-500' },
		raw_app: { rest: 'text-orange-500', hover: 'group-hover:text-orange-500' },
		script: { rest: 'text-blue-500', hover: 'group-hover:text-blue-500' },
		agent: { rest: 'text-violet-500', hover: 'group-hover:text-violet-500' }
	}

	let cls = $derived(
		muted ? `text-hint transition-colors ${COLOUR[kind].hover}` : COLOUR[kind].rest
	)
</script>

<!-- A flex box, as RowIcon's is: an inline one would take the line height of the text around it. -->
<span class="flex shrink-0 items-center justify-center" title={kind}>
	{#if kind === 'flow'}
		<BarsStaggered {size} class={cls} />
	{:else if kind === 'app' || kind === 'raw_app'}
		<LayoutDashboard {size} class={cls} />
	{:else if kind === 'agent'}
		<Bot {size} class={cls} />
	{:else}
		<Code2 {size} class={cls} />
	{/if}
</span>
