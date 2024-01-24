<script lang="ts">
	import ComponentCallbacks from './ComponentCallbacks.svelte'

	let componentCallbacks: ComponentCallbacks | undefined = undefined

	function keydown(event: KeyboardEvent) {
		// Ignore keydown events if the user is typing in monaco
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}
		switch (event.key) {
			case 'Escape':
				componentCallbacks?.handleEscape(event)
				break

			case 'ArrowUp': {
				componentCallbacks?.handleArrowUp(event)
				break
			}

			case 'ArrowDown': {
				componentCallbacks?.down(event)
				break
			}

			case 'ArrowRight': {
				componentCallbacks?.right(event)
				break
			}

			case 'ArrowLeft': {
				componentCallbacks?.left(event)
				break
			}

			case 'c':
				if (event.ctrlKey || event.metaKey) {
					componentCallbacks?.handleCopy(event)
				}
				break

			case 'x':
				if (event.ctrlKey || event.metaKey) {
					componentCallbacks?.handleCut(event)
				}
				break

			default:
				break
		}
	}
</script>

<ComponentCallbacks bind:this={componentCallbacks} />

<svelte:window on:keydown={keydown} on:paste={componentCallbacks?.handlePaste} />
