<script lang="ts">
	import { Button } from '$lib/components/common'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard } from '$lib/utils'
	import { Copy, KeyRound, RefreshCw } from 'lucide-svelte'
	import { onMount } from 'svelte'
	import Toggle from './Toggle.svelte'

	let loading = $state(true)
	let enabled = $state(false)
	let clientId = $state<string | undefined>(undefined)
	let tokenEndpoint = $state('')
	let generating = $state(false)
	// Plaintext secret returned once by the generate call — never re-fetchable.
	let newSecret = $state<string | undefined>(undefined)

	async function load() {
		loading = true
		try {
			const res = await SettingService.getScimOauthConfig()
			enabled = res.enabled
			clientId = res.client_id
			tokenEndpoint = res.token_endpoint
		} catch (e) {
			sendUserToast(`Failed to load SCIM OAuth config: ${e}`, true)
		} finally {
			loading = false
		}
	}

	async function generate() {
		generating = true
		try {
			const res = await SettingService.generateScimOauthSecret({
				requestBody: clientId ? { client_id: clientId } : {}
			})
			clientId = res.client_id
			tokenEndpoint = res.token_endpoint
			newSecret = res.client_secret
			enabled = true
			sendUserToast('Client secret generated — copy it now, it will not be shown again')
		} catch (e) {
			sendUserToast(`Failed to generate client secret: ${e}`, true)
		} finally {
			generating = false
		}
	}

	async function toggle(target: boolean) {
		if (target) {
			await generate()
		} else {
			try {
				await SettingService.disableScimOauthConfig()
				enabled = false
				clientId = undefined
				newSecret = undefined
				sendUserToast('OAuth client credentials disabled')
			} catch (e) {
				sendUserToast(`Failed to disable OAuth client credentials: ${e}`, true)
			}
		}
	}

	onMount(load)
</script>

<div class="flex flex-col gap-3">
	<div class="flex items-start justify-between gap-4">
		<div class="flex flex-col gap-1">
			<span class="text-sm font-semibold text-primary">OAuth 2.0 client credentials</span>
			<span class="text-xs text-secondary max-w-lg">
				Let an identity provider (e.g. Microsoft Entra ID) obtain short-lived access tokens via the
				OAuth 2.0 client-credentials grant instead of a static bearer token. Both methods stay
				valid.
			</span>
		</div>
		<Toggle
			disabled={loading || generating}
			checked={enabled}
			on:change={(e) => toggle(e.detail)}
			options={{ right: 'Enabled' }}
		/>
	</div>

	{#if enabled}
		<div class="flex flex-col gap-3 border rounded-md p-3 bg-surface-secondary">
			<div class="flex flex-col gap-1">
				<span class="text-2xs font-semibold text-secondary uppercase">Token endpoint</span>
				<div class="flex items-center gap-2">
					<code class="text-xs break-all grow">{tokenEndpoint}</code>
					<Button
						size="xs2"
						color="light"
						variant="border"
						startIcon={{ icon: Copy }}
						iconOnly
						on:click={() => copyToClipboard(tokenEndpoint)}
					/>
				</div>
			</div>

			<div class="flex flex-col gap-1">
				<span class="text-2xs font-semibold text-secondary uppercase">Client ID</span>
				<div class="flex items-center gap-2">
					<code class="text-xs break-all grow">{clientId ?? ''}</code>
					{#if clientId}
						<Button
							size="xs2"
							color="light"
							variant="border"
							startIcon={{ icon: Copy }}
							iconOnly
							on:click={() => copyToClipboard(clientId)}
						/>
					{/if}
				</div>
			</div>

			{#if newSecret}
				<div class="flex flex-col gap-1">
					<span class="text-2xs font-semibold text-secondary uppercase">Client secret</span>
					<div class="flex items-center gap-2">
						<code class="text-xs break-all grow text-primary">{newSecret}</code>
						<Button
							size="xs2"
							color="dark"
							startIcon={{ icon: Copy }}
							iconOnly
							on:click={() => copyToClipboard(newSecret)}
						/>
					</div>
					<span class="text-2xs text-orange-600 dark:text-orange-400">
						Copy this secret now — it will not be shown again. Regenerate to get a new one.
					</span>
				</div>
			{/if}

			<div class="flex gap-2">
				<Button
					size="xs"
					color="light"
					variant="border"
					startIcon={{ icon: RefreshCw }}
					disabled={generating}
					on:click={generate}
				>
					Regenerate secret
				</Button>
			</div>
		</div>
	{:else if !loading}
		<div class="flex">
			<Button
				size="xs"
				color="dark"
				startIcon={{ icon: KeyRound }}
				disabled={generating}
				on:click={generate}
			>
				Generate client credentials
			</Button>
		</div>
	{/if}
</div>
