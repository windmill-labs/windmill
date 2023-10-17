<script lang="ts">
	import { msToSec } from '$lib/utils'
	import type { Tweened } from 'svelte/motion'

	export let started_at_p: Tweened<number>
	export let len_p: Tweened<number>
	export let started_at: number | undefined
	export let duration_ms: number | undefined

	let i = 0
	let interval = setInterval((x) => {
		i++
		if (duration_ms) {
			clearInterval(interval)
		}
	}, 100)
</script>

<div style="width: {$started_at_p}%" class="h-4" /><div
	style="width: {$len_p}%"
	class="h-4 bg-blue-600 border center-center text-primary text-2xs whitespace-nowrap"
	>{Math.ceil($len_p)}% {#key i}{#if duration_ms}{msToSec(
				duration_ms,
				1
			)}s{:else if started_at}{msToSec(Date.now() - started_at, 1)}s{/if}{/key}
</div>
