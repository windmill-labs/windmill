<script lang="ts">
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

	let {
		id,
		description,
		onTrigger,
		children,
		showAnimation = true
	} = $props<{
		id: string | undefined
		description: string | undefined
		onTrigger?: (value?: string) => void // Function to call when the trigger is activated, if not provided, the component is discoverable for information purposes only
		children?: () => any
		showAnimation?: boolean
	}>()

	let isAnimating = $state(false)

	// Component is not discoverable if id or description is not provided
	const disabled = !id || !description

	function handleTrigger(value?: string) {
		if (disabled || !onTrigger) return
		if (!showAnimation) {
			onTrigger(value)
			return
		}
		isAnimating = true
		onTrigger(value)
		setTimeout(() => {
			isAnimating = false
		}, 2000)
	}

	$effect(() => {
		if (disabled) return

		// register the triggerable
		const currentId = id
		const currentData = { description, onTrigger: handleTrigger }
		const existingTriggerables = aiChatManager.triggerablesByAI
		existingTriggerables[currentId] = currentData

		return () => {
			// unregister the triggerable
			if (aiChatManager.triggerablesByAI[currentId]) {
				delete aiChatManager.triggerablesByAI[currentId]
			}
		}
	})
</script>

{#if disabled}
	{@render children?.()}
{:else}
	<div class="relative">
		{#if isAnimating}
			<div
				class="absolute -top-2.5 left-1/2 -translate-x-1/2 w-10 h-10 bg-blue-500/90 rounded-full z-[9999] pointer-events-none animate-ping"
			></div>
		{/if}
		{@render children?.()}
	</div>
{/if}
