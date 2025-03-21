<script lang="ts">
	import { createToggle, melt, createSync } from '@melt-ui/svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	const {
		elements: { root },
		states
	} = createToggle({
		onPressedChange: ({ curr, next }) => {
			if (next != curr) {
				dispatch('pressedChange', next)
			}
			return next
		}
	})

	export let pressed = false
	export let disabled = false

	const sync = createSync(states)
	$: sync.pressed(pressed, (v) => (pressed = Boolean(v)))
</script>

<button
	use:melt={$root}
	aria-label="Toggle italic"
	class=" items-center justify-center rounded-md
      data-[disabled]:cursor-not-allowed"
	{disabled}
>
	<slot {pressed} />
</button>
