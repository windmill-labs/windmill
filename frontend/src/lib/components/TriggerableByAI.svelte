<script lang="ts">
	import { triggerablesByAI } from '$lib/stores'

	let { id, description, onTrigger, children } = $props<{
		id: string
		description: string
		onTrigger?: (id: string) => void
		children?: () => any
	}>()

	// Use $effect for lifecycle management
	// The cleanup function returned from $effect will run on unmount
	$effect(() => {
		// Register this component
		triggerablesByAI.update((triggers) => {
			return { ...triggers, [id]: { description, onTrigger } }
		})

		// Return cleanup function that will run on unmount
		return () => {
			triggerablesByAI.update((triggers) => {
				const newTriggers = { ...triggers }
				delete newTriggers[id]
				return newTriggers
			})
		}
	})
</script>

{@render children?.()}
