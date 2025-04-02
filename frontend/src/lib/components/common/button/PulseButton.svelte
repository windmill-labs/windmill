<script>
	export let pulseDuration = 2
	export let numberOfPulses = 3
	export let scale = 1.2

	let isPulsing = false
	let pulseCount = 1

	export function triggerPulse(count) {
		if (pulseCount < 1) return
		if (!isPulsing) {
			pulseCount = count
			isPulsing = true
			setTimeout(() => {
				isPulsing = false
			}, pulseDuration * count * 1000)
		}
	}
</script>

<div class="relative">
	<div>
		<slot />
	</div>
	{#each Array(numberOfPulses) as _, i}
		<div
			class:deactivate={isPulsing}
			style={`--pulse-duration: ${pulseDuration}s; --i:${i}; --scale:${scale}; --number-of-pulses:${numberOfPulses}; --pulse-count:${pulseCount};`}
		>
			<div class="pulse" class:active={isPulsing}></div>
		</div>
	{/each}
</div>

<style>
	.relative {
		position: relative;
		display: inline-block;
	}

	.pulse {
		position: absolute;
		top: 50%;
		left: 50%;
		width: 100%;
		height: 100%;
		background: rgb(81, 151, 255);
		border-radius: 8px;
		transform: translate(-50%, -50%) scale(1);
		z-index: 0;
		pointer-events: none;
		opacity: 0;
	}

	.pulse.active {
		animation: pulse var(--pulse-duration) ease-out infinite;
		animation-delay: calc(var(--i) * var(--pulse-duration) / var(--number-of-pulses));
		opacity: 1;
	}

	.deactivate {
		animation: fade calc(var(--pulse-duration) * 0.3) ease-out;
		animation-delay: calc(var(--pulse-duration) * var(--pulse-count) - var(--pulse-duration) * 0.2);
		opacity: 1;
	}

	@keyframes fade {
		0% {
			opacity: 0.8;
		}
		100% {
			opacity: 0;
		}
	}

	@keyframes pulse {
		100% {
			transform: translate(-50%, -50%) scale(var(--scale));
			opacity: 0;
		}
	}
</style>
