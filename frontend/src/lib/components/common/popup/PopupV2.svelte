<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { clickOutside } from '$lib/utils'
	import { createFloatingActions, type ComputeConfig } from 'svelte-floating-ui'
	import { fly } from 'svelte/transition'

	interface Props {
		floatingConfig?: ComputeConfig
		open?: boolean
		target?: string | undefined
		button?: import('svelte').Snippet
		children?: import('svelte').Snippet<[{ close: () => void }]>
	}

	let {
		floatingConfig = {
			strategy: 'absolute',
			//@ts-ignore
			placement: 'bottom-center'
		},
		open = $bindable(false),
		target = undefined,
		button,
		children
	}: Props = $props()

	// export let containerClasses: string = 'rounded-lg shadow-md border p-4 bg-surface'
	// export let floatingClasses: string = ''
	const [floatingRef, floatingContent] = createFloatingActions(floatingConfig)

	function close() {
		open = false
	}
</script>

<div use:floatingRef>
	{@render button?.()}
</div>

<Portal name="popup-v2" {target}>
	{#if open}
		<div
			class="border rounded-lg shadow-lg bg-surface z5000"
			style="position:absolute"
			use:floatingContent
			transition:fly={{ duration: 100, y: -16 }}
		>
			<div
				use:clickOutside={{
					eventToListenName: 'pointerdown',
					onClickOutside: () => (open = false)
				}}
			>
				{@render children?.({ close: () => close() })}
			</div>
		</div>
	{/if}
</Portal>
