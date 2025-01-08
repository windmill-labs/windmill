<script>
	export let pulseDuration = 1000

	let pulseCount = 0
	let isPulsing = false

	export function triggerPulse(count) {
		pulseCount = count
		if (!isPulsing) {
			isPulsing = true
			startPulse()
		}
	}

	function startPulse() {
		const interval = setInterval(() => {
			if (pulseCount <= 0) {
				clearInterval(interval)
				isPulsing = false
			} else {
				pulseCount--
			}
		}, pulseDuration)
	}
</script>

<div class="relative">
	<div
		class={`pulse ${pulseCount > 0 ? 'active' : ''}`}
		style={`--pulse-duration: ${pulseDuration}ms`}
	/>
	<slot />
</div>

<style>
	.pulse {
		position: absolute;
		top: 50%;
		left: 50%;
		width: 100%;
		height: 100%;
		background: radial-gradient(circle, rgba(0, 98, 255, 0.4) 0%, transparent 100%);
		border-radius: 50%;
		transform: translate(-50%, -50%) scale(1);
		z-index: 0;
		pointer-events: none;
		opacity: 0;
	}

	.pulse.active {
		animation: pulse var(--pulse-duration) ease-in infinite;
		opacity: 1;
	}

	@keyframes pulse {
		0% {
			transform: translate(-50%, -50%) scale(1);
			opacity: 0.8;
		}

		100% {
			transform: translate(-50%, -50%) scale(4);
			opacity: 0;
		}
	}
</style>
