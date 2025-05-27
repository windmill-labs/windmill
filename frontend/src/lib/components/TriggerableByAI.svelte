<script lang="ts">
	import { triggerablesByAI } from '$lib/stores'

	let {
		id,
		description,
		onTrigger,
		children,
		disabled = false
	} = $props<{
		id: string
		description: string
		onTrigger: (value?: string) => void
		children?: () => any
		disabled?: boolean
	}>()

	// Track animation state
	let isAnimating = $state(false)

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
		}, 1200) // Animation duration
	}

	$effect(() => {
		if (disabled) return
		triggerablesByAI.update((triggers) => {
			return { ...triggers, [id]: { description, onTrigger: handleTrigger } }
		})

		return () => {
			triggerablesByAI.update((triggers) => {
				const newTriggers = { ...triggers }
				delete newTriggers[id]
				return newTriggers
			})
		}
	})
</script>

<div class="ai-triggerable-wrapper">
	{#if isAnimating}
		<div class="ai-triggerable-animation"></div>
	{/if}
	<div class="ai-triggerable-content">
		{@render children?.()}
	</div>
</div>

<style>
	.ai-triggerable-wrapper {
		position: relative;
	}

	.ai-triggerable-content {
		/* This preserves original styling of children */
		display: contents;
	}

	.ai-triggerable-animation {
		position: absolute;
		top: -20px;
		left: 50%;
		transform: translateX(-50%);
		width: 40px;
		height: 40px;
		background-color: rgba(66, 133, 244, 0.9);
		border-radius: 50%;
		z-index: 9999;
		pointer-events: none;
		animation: pulse 0.6s ease-out forwards;
	}

	@keyframes pulse {
		0% {
			transform: translateX(-50%) scale(0);
			opacity: 1;
		}
		100% {
			transform: translateX(-50%) scale(2.5);
			opacity: 0;
		}
	}
</style>
