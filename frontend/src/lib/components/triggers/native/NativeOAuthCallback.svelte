<script lang="ts">
	import { onMount } from 'svelte'
	import { page } from '$app/state'
	import { WorkspaceIntegrationService } from '$lib/gen'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { decodeNativeOAuthState, postNativeOAuthResult } from './utils'

	let outcome = $state<{ ok: true; path: string } | { ok: false; error: string } | undefined>()

	async function finish(): Promise<{ ok: true; path: string } | { ok: false; error: string }> {
		const error = page.url.searchParams.get('error')
		const code = page.url.searchParams.get('code')
		const state = page.url.searchParams.get('state')
		if (error || !code || !state) {
			return { ok: false, error: error ?? 'The provider returned no authorization code' }
		}
		try {
			// The signed state carries the workspace and service it was issued for, so this page works
			// whatever workspace is selected and wherever the provider sends the user back.
			const decoded = decodeNativeOAuthState(state)
			if (!decoded) {
				return { ok: false, error: 'The provider returned an unrecognized state' }
			}
			const { workspace, service } = decoded
			const path = await WorkspaceIntegrationService.nativeTriggerServiceCallback({
				workspace,
				serviceName: service,
				requestBody: { code, state }
			})
			return { ok: true, path }
		} catch (err: any) {
			return { ok: false, error: err.body ?? err.message ?? String(err) }
		}
	}

	onMount(async () => {
		outcome = await finish()
		postNativeOAuthResult(outcome)
		if (window.opener) {
			window.close()
		}
	})
</script>

<div class="max-w-lg mx-auto mt-16 px-4">
	{#if outcome === undefined}
		<div class="flex items-center gap-2 text-sm text-secondary">
			<Loader2 class="animate-spin" size={16} />
			Finishing the connection...
		</div>
	{:else if outcome.ok}
		<Alert type="success" title="Account connected">
			Saved as {outcome.path}. You can close this window.
		</Alert>
	{:else}
		<Alert type="error" title="Could not connect the account">{outcome.error}</Alert>
	{/if}
</div>
