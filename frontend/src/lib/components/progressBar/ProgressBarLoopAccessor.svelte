<script lang="ts">
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { Tweened } from 'svelte/motion'
	import type { ProgressStateStoreValue } from './model'

	export let progress: Tweened<number>
	export let index: number
	export let length: number
	const state = getContext<Writable<ProgressStateStoreValue>>('state')

	$: isLast = index === length - 1
</script>

<div
	class="h-full absolute left-0 bottom-0 overflow-hidden {$state.error
		? 'bg-red-400'
		: 'bg-blue-400'}"
	style="width: {$progress}%; opacity: {isLast ? 100 : 100 / length}%"
/>
