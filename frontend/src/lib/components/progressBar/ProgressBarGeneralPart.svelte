<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import type { GeneralState } from './model'

	export let step: GeneralState
	export let finished = false
	export let duration = 200
	const progress = tweened(0, {
		duration,
		easing: cubicOut
	})

	$: progress.set(finished || step.isDone ? 100 : 0)
</script>

<div class="absolute left-0 bottom-0 h-full bg-blue-400" style="width: {$progress}%" />
