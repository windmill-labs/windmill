import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

export interface TriggerableByAIOptions {
	id?: string
	description?: string
	callback?: (value?: string) => void
	disabled?: boolean
	showAnimation?: boolean
}

export function triggerableByAI(element: HTMLElement, options: TriggerableByAIOptions = {}) {
	let { id, description, callback, disabled = false, showAnimation = true } = options

	// Component is not discoverable if id or description is not provided
	const isDisabled = disabled || !id || !description

	function createPulseEffect() {
		if (!showAnimation) return

		// Get the bounding rect of the element
		const rect = element.getBoundingClientRect()
		if (rect.width === 0 && rect.height === 0) return // Skip if no dimensions

		const centerX = rect.left + rect.width / 2
		const centerY = rect.top + rect.height / 2

		// Create pulse element
		const pulse = document.createElement('div')
		pulse.className = 'fixed w-10 h-10 bg-blue-500/90 rounded-full pointer-events-none animate-ping'
		pulse.style.cssText = `
			left: ${centerX - 20}px;
			top: ${centerY - 20}px;
			z-index: 9999;
		`

		// Add to body
		document.body.appendChild(pulse)

		// Remove after animation
		setTimeout(() => {
			if (pulse.parentNode) {
				pulse.parentNode.removeChild(pulse)
			}
		}, 2000)
	}

	function handleTrigger(value?: string) {
		if (!callback) return
		if (!showAnimation) {
			callback(value)
			return
		}

		createPulseEffect()
		callback(value)
	}

	function register() {
		if (isDisabled || !id || !description) return

		// register the triggerable
		const currentData = { description, onTrigger: handleTrigger }
		aiChatManager.triggerablesByAI[id] = currentData
	}

	function unregister() {
		if (isDisabled || !id) return

		// unregister the triggerable
		if (aiChatManager.triggerablesByAI[id]) {
			delete aiChatManager.triggerablesByAI[id]
		}
	}

	// Initial registration
	register()

	return {
		update(newOptions: TriggerableByAIOptions) {
			// Unregister with old options
			unregister()

			// Update options
			;({ id, description, callback, disabled = false, showAnimation = true } = newOptions)

			// Re-register with new options
			register()
		},
		destroy() {
			unregister()
		}
	}
}