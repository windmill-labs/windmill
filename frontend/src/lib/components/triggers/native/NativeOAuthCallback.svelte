<script lang="ts">
	import { onMount } from 'svelte'
	import { page } from '$app/state'
	import { WorkspaceIntegrationService } from '$lib/gen'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { decodeNativeOAuthState, NATIVE_TRIGGER_SERVICES } from './utils'

	/** Finishes a connect made with a workspace OAuth app: exchanges the code, then hands the
	 * token to the resource connect drawer that opened this window, which saves it. */
	let error = $state<string | undefined>()
	let done = $state(false)

	async function finish() {
		const providerError = page.url.searchParams.get('error')
		const code = page.url.searchParams.get('code')
		const state = page.url.searchParams.get('state')
		if (providerError || !code || !state) {
			return {
				type: 'error',
				error: providerError ?? 'The provider returned no authorization code'
			}
		}
		// The signed state carries the workspace and service it was issued for, so this page works
		// whatever workspace is selected and wherever the provider sends the user back.
		const decoded = decodeNativeOAuthState(state)
		if (!decoded) {
			return { type: 'error', error: 'The provider returned an unrecognized state' }
		}
		try {
			const res = await WorkspaceIntegrationService.nativeTriggerServiceCallback({
				workspace: decoded.workspace,
				serviceName: decoded.service,
				requestBody: { code, state }
			})
			return {
				type: 'success',
				res,
				resource_type: NATIVE_TRIGGER_SERVICES[decoded.service].resourceType
			}
		} catch (err: any) {
			return { type: 'error', error: err.body ?? err.message ?? String(err) }
		}
	}

	onMount(async () => {
		const message = await finish()
		if (message.type === 'error') error = message.error
		else done = true
		if (window.opener) {
			window.opener.postMessage(message, window.location.origin)
			window.close()
		} else {
			localStorage.setItem('oauth-callback', JSON.stringify(message))
		}
	})
</script>

<div class="max-w-lg mx-auto mt-16 px-4">
	{#if error}
		<Alert type="error" title="Could not connect the account">{error}</Alert>
	{:else if done}
		<Alert type="success" title="Account connected">
			Return to the window you connected from to save it.
		</Alert>
	{:else}
		<div class="flex items-center gap-2 text-sm text-secondary">
			<Loader2 class="animate-spin" size={16} />
			Finishing the connection...
		</div>
	{/if}
</div>
