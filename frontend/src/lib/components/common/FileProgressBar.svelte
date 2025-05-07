<script lang="ts">
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'

	export let color = 'blue'
	export let progress = 0
	export let ended: boolean = false

	const tweenedProgress = tweened(progress, {
		duration: 400,
		easing: cubicOut
	})

	$: tweenedProgress.set(progress)
</script>

{#key color}
	<div class="flex flex-row gap-1 items-center justify-between w-full">
		{#if ended}
			<slot />
		{:else}
			<div class="progress-bar">
				<div
					class={`progress ${!ended ? 'blinking' : ''}`}
					style={`--color: ${color}; width: ${Math.round($tweenedProgress)}%`}
				></div>
			</div>
			<span class="text-xs p-1">{Math.round($tweenedProgress)}%</span>
		{/if}
	</div>
{/key}

<style>
	.progress-bar {
		height: 10px;
		background-color: lightgray;
		border-radius: 5px;
		overflow: hidden;
		width: 100%;
	}
	.progress {
		height: 100%;
		background-color: var(--color);
		transition: width 0.4s ease-out;
	}
	.blinking {
		animation: blink 1s linear infinite;
	}
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}
</style>
