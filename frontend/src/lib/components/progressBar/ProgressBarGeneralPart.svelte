<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'

	export let isDone: boolean = false
	export let duration = 200
	const progress = tweened(isDone ? 100 : 0, {
		duration,
		easing: cubicOut
	})

	$: isDone ? progress.set(100) : progress.set(0)
</script>

<div class="absolute left-0 bottom-0 h-full bg-blue-400" style="width: {$progress}%" />
