<script lang="ts">
	import { page } from '$app/stores'

	import { SvelteToast } from '@zerodevx/svelte-toast'
	import '$lib/assets/app.css'
	interface Props {
		children?: import('svelte').Snippet
	}

	let { children }: Props = $props()

	// Default toast options
	const toastOptions = {
		duration: 5000, // duration of progress bar tween to the `next` value
		initial: 1, // initial progress bar value
		next: 0, // next progress value
		pausable: true, // pause progress bar tween on mouse hover
		dismissable: true, // allow dismiss with close button
		reversed: false, // insert new toast to bottom of stack
		intro: { x: 256 }, // toast intro fly animation settings
		theme: {} // css var overrides
	}

	document.getElementById('svelte-global-loader')?.remove()
</script>

<svelte:head>
	<!-- {#if !import.meta.env.PROD}
		<script
			data-project-id="JLtc2teApu4xqh47avxGx7oTAwY0rqAdVTqsV8MC"
			data-is-production-environment="false"
			src="https://snippet.meticulous.ai/v1/meticulous.js"
		></script>
	{/if} -->
	<title>{$page.data?.stuff?.title ? `${$page.data?.stuff?.title} | ` : ''}Windmill</title>
</svelte:head>

{@render children?.()}

<div class="wrap">
	<SvelteToast options={toastOptions} />
</div>

<style>
	.wrap {
		display: contents;
		font-family: 'Inter', sans-serif;
		font-size: 0.875rem;
	}
	.wrap :global(strong) {
		font-weight: 600;
	}
</style>
