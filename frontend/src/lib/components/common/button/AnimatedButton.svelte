<script lang="ts">
	import AnimatedButtonInner from './AnimatedButtonInner.svelte'
	export let marginWidth = '2px'
	export let animationDuration = '2s'
	export let baseRadius = '4px'
	export let animate = true
	export let wrapperClasses = ''
	export let ringColor = 'transparent'
	export let darkMode = false
</script>

{#if animate}
	<AnimatedButtonInner
		{marginWidth}
		{animationDuration}
		{baseRadius}
		{animate}
		{wrapperClasses}
		{ringColor}
		{darkMode}
	>
		<slot />
	</AnimatedButtonInner>
{:else}
	<div class={wrapperClasses}>
		<slot />
	</div>
{/if}
