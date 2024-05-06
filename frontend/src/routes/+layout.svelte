<script lang="ts">
	import { page } from '$app/stores'

	import { Toaster } from 'svelte-sonner'
	import '$lib/assets/app.css'
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

<div class="wrap">
	<Toaster position="top-right" richColors closeButton />
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
