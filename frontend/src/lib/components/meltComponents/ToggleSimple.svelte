<script lang="ts">
	import { createToggle, melt, createSync } from '@melt-ui/svelte'

	const {
		elements: { root },
		states
	} = createToggle()

	export let pressed = false

	const sync = createSync(states)
	$: sync.pressed(pressed, (v) => (pressed = Boolean(v)))
</script>

<button
	use:melt={$root}
	aria-label="Toggle italic"
	class=" items-center justify-center rounded-md
     hover:bg-surface-hover
      data-[disabled]:cursor-not-allowed data-[state=on]:bg-surface-hover
      data-[state=on]:text-blue-500"
>
	<slot {pressed} />
</button>
