<script lang="ts">
	import { createPopperActions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from '$lib/components/Portal.svelte'

	import { fade } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

	export async function refresh() {
		await getInstance()?.update()
	}

	interface Props {
		placement?: PopoverPlacement
		class?: string
		showTooltip?: boolean
		children?: import('svelte').Snippet
		content?: import('svelte').Snippet
	}

	let {
		placement = 'bottom',
		class: clazz = '',
		showTooltip = $bindable(false),
		children,
		content
	}: Props = $props()

	const [popperRef, popperContent, getInstance] = createPopperActions({ placement })

	export function open() {
		showTooltip = true
	}
	export function close() {
		showTooltip = false
	}
</script>

<fragment use:popperRef>
	{@render children?.()}
</fragment>
{#if showTooltip}
	<Portal name="manual-popover">
		<div
			use:popperContent
			class={twMerge('z-[901] rounded-lg shadow-md border p-4 bg-surface', clazz)}
			transition:fade={{ duration: 200 }}
		>
			{@render content?.()}
		</div>
	</Portal>
{/if}
