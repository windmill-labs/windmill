<script lang="ts">
	import { Button } from './common'
	import Alert from './common/alert/Alert.svelte'

	// Shown instead of runnable-authored markup on the public app surfaces, where the
	// app's author and its viewer are different people and the app is not sandboxed.
	// Enabling the app's sandbox isolates the markup and removes this prompt.
	let { children }: { children: import('svelte').Snippet } = $props()
	let approved = $state(false)
</script>

{#if approved}
	{@render children()}
{:else}
	<Alert type="error" title="This content is not rendered">
		<div class="flex flex-col items-start gap-2">
			<p>
				Rendering it can expose you to <a
					href="https://owasp.org/www-community/attacks/xss/"
					target="_blank"
					rel="noreferrer"
					class="underline">XSS attacks</a
				>. Only enable it if you trust the author of this app.
			</p>
			<Button unifiedSize="sm" variant="default" on:click={() => (approved = true)}>
				Enable rendering
			</Button>
		</div>
	</Alert>
{/if}
