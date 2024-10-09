<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { clickOutside } from '$lib/utils'
	import { createFloatingActions, type ComputeConfig } from 'svelte-floating-ui'

	export let floatingConfig: ComputeConfig = {
		strategy: 'absolute',
		//@ts-ignore
		placement: 'bottom-center'
	}
	export let open = false
	export let target: string | undefined = undefined

	// export let containerClasses: string = 'rounded-lg shadow-md border p-4 bg-surface'
	// export let floatingClasses: string = ''
	const [floatingRef, floatingContent] = createFloatingActions(floatingConfig)

	function close(div: Element | null) {
		open = false
	}

	let acceptClickoutside = false
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
	<slot {pointerup} {pointerdown} name="button" />
</div>

<Portal name="popup-v2" {target}>
	{#if open}
		<div
			class="border rounded-lg shadow-lg bg-surface z5000"
			style="position:absolute"
			use:floatingContent
		>
			<div
				use:clickOutside
				on:click_outside={() => {
					if (acceptClickoutside) {
						acceptClickoutside = false
						open = false
					}
				}}
			>
				<slot {close} />
			</div>
		</div>
	{/if}
</Portal>
