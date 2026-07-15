<script lang="ts">
	import { Button } from './common'

	// Shown instead of runnable-authored markup on the public app surfaces, where the
	// app's author and its viewer are different people and the app is not sandboxed.
	// Enabling the app's sandbox isolates the markup and removes this prompt.
	let { children }: { children: import('svelte').Snippet } = $props()
	let approved = $state(false)
</script>

{#if approved}
	{@render children()}
{:else}
	<div class="font-main text-sm">
		<div class="flex flex-col">
			<div class="bg-red-400 py-1 rounded-t text-white font-bold text-center">Warning</div>
			<p
				class="text-primary mb-2 text-left border-2 !border-t-0 rounded-b border-red-400 overflow-auto p-1"
				>Rendering this content can expose you to <a
					href="https://owasp.org/www-community/attacks/xss/"
					target="_blank"
					rel="noreferrer"
					class="hover:underline">XSS attacks</a
				>. Only enable it if you trust the author of the app.
			</p>
		</div>
		<div class="center-center">
			<Button unifiedSize="md" variant="default" on:click={() => (approved = true)}>
				Enable rendering
			</Button>
		</div>
	</div>
{/if}
