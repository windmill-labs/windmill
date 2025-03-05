<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	export let marginWidth = '2px'
	export let animationDuration = '2s'
	export let baseRadius = '4px'
	export let animate = true
	export let wrapperClasses = ''
	export let ringColor = 'transparent'
	export let darkMode = false

	const gradientColors = {
		light: ['#d6e5ff', '#0073ff', '#5aa2fa', '#0272fa', '#d6e5ff'],
		dark: ['#0469db', '#15498a', '#031ea3', '#0073ff', '#0469db']
	}

	let clientWidth = 0
	let clientHeight = 0

	$: circleRadius = Math.ceil(
		Math.sqrt(clientWidth * clientWidth + clientHeight * clientHeight) / 2
	)

	$: gradientString = `from 0deg, ${gradientColors[darkMode ? 'dark' : 'light'].join(', ')}`
</script>

<DarkModeObserver bind:darkMode />

<div
	class={twMerge('gradient-button', wrapperClasses)}
	style="--margin-width: {marginWidth}; --animation-duration: {animationDuration}; --base-radius: {baseRadius}; --circle-radius: {circleRadius}; --ring-color: {ringColor}; --gradient: {gradientString}"
	class:animate
	bind:clientWidth
	bind:clientHeight
>
	<slot />
</div>

<style>
	.gradient-button {
		position: relative;
		font-size: inherit;
		border: none;
		border-radius: calc(var(--base-radius) + var(--margin-width, 2px));
		color: currentColor;
		background: inherit;
		z-index: 1;
		overflow: hidden;
		padding: var(--margin-width, 1px);
	}

	/* Circular gradient */
	.gradient-button::before {
		content: '';
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: calc(var(--circle-radius, 300px) * 2px);
		height: calc(var(--circle-radius, 300px) * 2px);
		background: var(--ring-color, transparent);
		border-radius: 50%;
		z-index: -1;
		animation: none;
	}

	.gradient-button.animate::before {
		background: conic-gradient(var(--gradient));
		animation: rotate var(--animation-duration, 2s) linear infinite;
	}

	/* inner background */
	.gradient-button::after {
		content: '';
		position: absolute;
		top: var(--margin-width, 2px);
		right: var(--margin-width, 2px);
		bottom: var(--margin-width, 2px);
		left: var(--margin-width, 2px);
		background: inherit;
		border-radius: var(--base-radius);
		z-index: -1;
	}

	@keyframes rotate {
		from {
			transform: translate(-50%, -50%) rotate(0deg);
		}
		to {
			transform: translate(-50%, -50%) rotate(360deg);
		}
	}
</style>
