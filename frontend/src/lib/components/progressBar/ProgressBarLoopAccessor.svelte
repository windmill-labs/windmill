<script lang="ts">
	import type { Tweened } from 'svelte/motion'

	export let progress: Tweened<number>
	export let index: number
	export let length: number
	$: isLast = index === length - 1
</script>

<div
	class="h-full absolute left-0 bg-blue-400 overflow-hidden"
	style="width: {$progress}%; opacity: {isLast ? 100 : 100 / length}%"
/>
