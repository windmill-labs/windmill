<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'

	const progress = tweened(0, {
		duration: 400,
		easing: cubicOut
	})

	export let isFirst: boolean
	export let isLast: boolean
	export let sumUpTo: number
	export let length: number

	export let shouldToggle: boolean

	$: shouldToggle && progress.set(length)
</script>

<div
	class={`bg-blue-200 h-2.5 absolute ${isFirst ? 'rounded-l-full' : ''} ${
		isLast ? 'rounded-r-full' : ''
	}`}
	style={`${isFirst ? 'left:0%;' : `left:${sumUpTo}%;`} width: ${$progress}%`}
/>
