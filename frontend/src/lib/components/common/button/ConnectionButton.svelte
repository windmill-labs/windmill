<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { pointerDownOutside } from '$lib/utils'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		isOpen?: boolean
		openConnection: () => void
		closeConnection: () => void
		btnWrapperClasses?: string
		id?: string | undefined
		small?: boolean
	}

	let {
		isOpen = false,
		openConnection,
		closeConnection,
		btnWrapperClasses = '',
		id = undefined,
		small
	}: Props = $props()

	let selected = $state(false)

	const { panzoomActive } = getContext<AppViewerContext>('AppViewerContext')

	async function getConnectionButtonElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-connection-button], [data-connection-button] *')
		) as HTMLElement[]
	}

	function handleConnect(activate = false) {
		if (isOpen) {
			deactivateConnection()
			return
		}
		if (activate) {
			activateConnection()
		}
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

	function handlePointerDownOutside() {
		if (!$panzoomActive) {
			deactivateConnection()
		}
	}

	$effect(() => {
		!isOpen && (selected = false)
	})
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: isOpen,
		exclude: getConnectionButtonElements,
		customEventName: 'pointerdown_connecting',
		onClickOutside: () => handlePointerDownOutside()
	}}
	onkeydown={handleKeyDown}
	data-connection-button
>
	<AnimatedButton
		animate={isOpen && selected}
		baseRadius="6px"
		animationDuration="2s"
		marginWidth="2px"
	>
		<Button
			size={small ? 'xs' : 'md'}
			variant="default"
			title="Connect"
			on:click={() => handleConnect(true)}
			{id}
			wrapperClasses={twMerge(btnWrapperClasses, selected ? 'opacity-100' : '')}
			btnClasses={twMerge(
				small ? 'p-0' : '',
				isOpen && selected ? 'bg-surface hover:bg-surface' : ''
			)}
		>
			<Plug size={14} />
		</Button>
	</AnimatedButton>
</div>
