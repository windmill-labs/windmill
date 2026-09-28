<script lang="ts">
	import { onMount } from 'svelte'
	import { page } from '$app/state'
	import { WorkspaceIntegrationService } from '$lib/gen'
	import type { NativeServiceName } from '$lib/gen/types.gen'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { postNativeOAuthResult } from './utils'

	interface Props {
		/** The exact redirect URI the authorization request was made with. */
		redirectUri: string
	}

	let { redirectUri }: Props = $props()

	let outcome = $state<{ ok: true; path: string } | { ok: false; error: string } | undefined>()

	// The signed state carries the workspace and service it was issued for, so this page works
	// whatever workspace is selected and wherever the provider sends the user back.
	function decodeState(state: string): { workspace: string; service: NativeServiceName } {
		const payload = atob(state.split(':')[0].replace(/-/g, '+').replace(/_/g, '/'))
		const [workspace, service] = payload.split(':')
		return { workspace, service: service as NativeServiceName }
	}

	async function finish(): Promise<{ ok: true; path: string } | { ok: false; error: string }> {
		const error = page.url.searchParams.get('error')
		const code = page.url.searchParams.get('code')
		const state = page.url.searchParams.get('state')
		if (error || !code || !state) {
			return { ok: false, error: error ?? 'The provider returned no authorization code' }
		}
		try {
			const { workspace, service } = decodeState(state)
			const path = await WorkspaceIntegrationService.nativeTriggerServiceCallback({
				workspace,
				serviceName: service,
				requestBody: { code, state, redirect_uri: redirectUri }
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
