import { onMount, type EventDispatcher } from 'svelte'

/**
 * This wrapper emulates the behavior of an event dispatcher in Svelte 4
 *
 * A lot of svelte 4 code depended on events not being picked up when they
 * were dispatched before being mounted. In svelte 5, these events are
 * being reacted to which cause infinite loops.
 */
export function createDispatcherIfMounted<EventMap extends Record<string, any> = any>(
	dispatch: EventDispatcher<EventMap>
): EventDispatcher<EventMap> {
	let mounted = false
	onMount(() => (mounted = true))
	return ((...args: Parameters<typeof dispatch>) => {
		if (!mounted) return false
		return dispatch(...args)
	}) as any
}
