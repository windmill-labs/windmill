<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { pointerDownOutside } from '$lib/utils'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'

	export let isOpen = false
	export let openConnection: () => void
	export let closeConnection: () => void

	let selected = false

	const { panzoomActive } = getContext<AppViewerContext>('AppViewerContext')

	async function getConnectionButtonElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-connection-button], [data-connection-button] *')
		) as HTMLElement[]
	}

	function handleConnect() {
		if (isOpen) {
			deactivateConnection()
			return
		}
		activateConnection()
	}

	function activateConnection() {
		selected = true
		openConnection()
	}

	function deactivateConnection() {
		selected = false
		closeConnection()
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation()
			e.preventDefault()
			handleConnect()
		}
	}

	function handlePointerDownOutside(e: CustomEvent) {
		if (!$panzoomActive) {
			deactivateConnection()
		}
	}

	$: !isOpen && (selected = false)
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: isOpen,
		exclude: getConnectionButtonElements,
		customEventName: 'pointerdown_connecting'
	}}
	on:keydown={handleKeyDown}
	on:pointerdown_outside={handlePointerDownOutside}
	data-connection-button
>
	<AnimatedButton
		animate={isOpen && selected}
		baseRadius="6px"
		animationDuration="2s"
		marginWidth="2px"
	>
		<Button
			size="xs"
			variant="border"
			color="light"
			title="Connect"
			on:click={handleConnect}
			id="schema-plug"
		>
			<Plug size={14} />
		</Button>
	</AnimatedButton>
</div>
