<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { clickOutside } from '$lib/utils'
	import { createFloatingActions, type ComputeConfig } from 'svelte-floating-ui'

	interface Props {
		floatingConfig?: ComputeConfig
		open?: boolean
		target?: string | undefined
		button?: import('svelte').Snippet<[any]>
		children?: import('svelte').Snippet<[any]>
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

	function close(div: Element | null) {
		open = false
	}

	let acceptClickoutside = $state(false)
	function pointerup() {
		setTimeout(() => {
			acceptClickoutside = true
		}, 100)
	}

	function pointerdown() {
		if (acceptClickoutside && open) {
			open = false
		} else {
			acceptClickoutside = false
			open = true
		}
	}
</script>

<div use:floatingRef>
	{@render button?.({ pointerup, pointerdown })}
</div>

<Portal name="popup-v2" {target}>
	{#if open}
		<div
			class="border rounded-lg shadow-lg bg-surface z5000"
			style="position:absolute"
			use:floatingContent
		>
			<!-- svelte-ignore event_directive_deprecated -->
			<div
				use:clickOutside
				on:click_outside={() => {
					if (acceptClickoutside) {
						acceptClickoutside = false
						open = false
					}
				}}
			>
				{@render children?.({ close })}
			</div>
		</div>
	{/if}
</Portal>
