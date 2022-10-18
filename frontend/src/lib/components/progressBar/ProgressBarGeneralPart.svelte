<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'

	export let isDone: boolean = false
	const progress = tweened(isDone ? 100 : 0, {
		duration: 400,
		easing: cubicOut
	})

	$: isDone ? progress.set(100) : progress.set(0)
</script>

<div class="h-full bg-blue-400" style="width: {$progress}%" />
