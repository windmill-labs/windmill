<script lang="ts">
	import { AIChatService } from '$lib/components/copilot/chat/AIChatManager.svelte'

	let { id, description, onTrigger, children } = $props<{
		id: string | undefined
		description: string | undefined
		onTrigger?: (value?: string) => void // Function to call when the trigger is activated, if not provided, the component is discoverable for information purposes only
		children?: () => any
	}>()

	// Track animation state
	let isAnimating = $state(false)

	// Component is not discoverable if id or description is not provided
	const disabled = !id || !description

	// Wrapper for onTrigger that adds animation
	function handleTrigger(value?: string) {
		if (disabled || !onTrigger) return

		// Show animation
		isAnimating = true

		// Call the actual onTrigger
		onTrigger(value)

		// Reset animation state after animation completes
		setTimeout(() => {
			isAnimating = false
		}, 2000) // Animation duration
	}

	$effect(() => {
		if (disabled) return

		const currentId = id
		const currentData = { description, onTrigger: handleTrigger }

		const existingTriggerables = AIChatService.triggerablesByAI
		existingTriggerables[currentId] = currentData

		return () => {
			if (AIChatService.triggerablesByAI[currentId]) {
				delete AIChatService.triggerablesByAI[currentId]
			}
		}
	})
</script>

{#if disabled}
	{@render children?.()}
{/if}

{#if !disabled}
	<div class="ai-triggerable-wrapper">
		{#if isAnimating}
			<div class="ai-triggerable-animation"></div>
		{/if}
		<div class="ai-triggerable-content">
			{@render children?.()}
		</div>
	</div>
{/if}

<style>
	.ai-triggerable-wrapper {
		position: relative;
	}

	.ai-triggerable-content {
		display: contents;
		height: 100%;
	}

	.ai-triggerable-animation {
		position: absolute;
		top: -10px;
		left: 50%;
		transform: translateX(-50%);
		width: 40px;
		height: 40px;
		background-color: rgba(66, 133, 244, 0.9);
		border-radius: 50%;
		z-index: 9999;
		pointer-events: none;
		animation: pulse 1.5s ease-out forwards;
	}

	@keyframes pulse {
		0% {
			transform: translateX(-50%) scale(0);
			opacity: 1;
		}
		100% {
			transform: translateX(-50%) scale(3);
			opacity: 0;
		}
	}
</style>
