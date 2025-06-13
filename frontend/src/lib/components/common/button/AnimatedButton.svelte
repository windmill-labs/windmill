<script lang="ts">
	import AnimatedButtonInner from './AnimatedButtonInner.svelte'
	interface Props {
		marginWidth?: string
		animationDuration?: string
		baseRadius?: string
		animate?: boolean
		wrapperClasses?: string
		ringColor?: string
		darkMode?: boolean
		children?: import('svelte').Snippet
	}

	let {
		marginWidth = '2px',
		animationDuration = '2s',
		baseRadius = '4px',
		animate = true,
		wrapperClasses = '',
		ringColor = 'transparent',
		darkMode = false,
		children
	}: Props = $props()
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
		{@render children?.()}
	</AnimatedButtonInner>
{:else}
	<div class={wrapperClasses}>
		{@render children?.()}
	</div>
{/if}
