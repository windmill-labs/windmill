<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { clickOutside, pointerDownOutside } from '$lib/utils'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let isOpen = false
	export let openConnection: () => void
	export let closeConnection: () => void

	let selected = false

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

	$: !isOpen && (selected = false)
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	use:clickOutside={{
		capture: true,
		stopPropagation: isOpen,
		exclude: getConnectionButtonElements
	}}
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: isOpen,
		exclude: getConnectionButtonElements
	}}
	on:keydown|preventDefault|stopPropagation={(e) => e.key === 'Escape' && handleConnect()}
	on:pointerdown_outside={deactivateConnection}
	on:click_outside={deactivateConnection}
	class="connection-access"
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
