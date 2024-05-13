<script lang="ts">
	import { page } from '$app/stores'

	import { Toaster } from 'svelte-sonner'
	import '$lib/assets/app.css'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'

	let darkMode = false
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

<slot />

<DarkModeObserver bind:darkMode />

<div class="wrap">
	<Toaster position="top-right" richColors closeButton theme={darkMode ? 'dark' : 'light'} />
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

	:global([data-sonner-toaster][data-theme='dark']) {
		--info-bg: #eaeef4 !important;
		--info-border: #3b82f6 !important;
		--info-text: #3b82f6 !important;

		--error-bg: #fef2f2 !important;
		--error-border: #ef4444 !important;
		--error-text: #ef4444 !important;
	}
</style>
