<script lang="ts">
	import type { KeyboardEventHandler } from 'svelte/elements'
	import { overlayHostActive } from './overlayHost.svelte'

	type KeyHandler = KeyboardEventHandler<Window>

	// Use this instead of `<svelte:window onkeydown>` in anything that can render inside a
	// session preview tab: hidden tabs stay mounted, and their window listeners would
	// otherwise act on an editor the user cannot see.
	let { onkeydown, onkeydowncapture }: { onkeydown?: KeyHandler; onkeydowncapture?: KeyHandler } =
		$props()

	const hostActive = overlayHostActive()
</script>

<svelte:window
	onkeydown={(e) => hostActive() && onkeydown?.(e)}
	onkeydowncapture={(e) => hostActive() && onkeydowncapture?.(e)}
/>
