<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Alert } from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { Check, X, ExternalLink, Cog, Plug } from 'lucide-svelte'
	import { NextcloudIcon } from '$lib/components/icons'
	import { WorkspaceIntegrationService, type NativeServiceName } from '$lib/gen'
	import OAuthClientConfig from './OAuthClientConfig.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'

	interface WorkspaceIntegration {
		service_name: string
		oauth_data: {
			client_id: string
			client_secret: string
			base_url: string
			redirect_uri: string
		} | null
	}

	interface ServiceConfig {
		name: string
		displayName: string
		description: string
		icon: any
		docsUrl?: string
	}

	const supportedServices: Record<string, ServiceConfig> = {
		nextcloud: {
			name: 'nextcloud',
			displayName: 'Nextcloud',
			description: 'Connect to Nextcloud for file operations and webhook triggers',
			icon: NextcloudIcon,
			docsUrl: 'https://www.windmill.dev/docs/integrations/nextcloud'
		}
	}

	let integrations = $state<WorkspaceIntegration[]>([])
	let loading = $state(false)
	let connecting = $state<string | null>(null)
	let showingConfig = $state<string | null>(null)
	let processingCallback = $state(false)

	async function loadIntegrations() {
		if (!$workspaceStore) return

		loading = true
		try {
			const response = await WorkspaceIntegrationService.listNativeTriggerServices({
				workspace: $workspaceStore
			})
			integrations = response.map((item) => ({
				service_name: item.service_name,
				oauth_data: item.oauth_data || null
			}))
		} catch (err: any) {
			console.error('Failed to load workspace integrations:', err)
			sendUserToast(`Failed to load integrations: ${err.message}`, true)
		} finally {
			loading = false
		}
	}

	async function deleteIntegration(serviceName: string) {
		if (!$workspaceStore) return

		try {
			await WorkspaceIntegrationService.deleteNativeTriggerService({
				workspace: $workspaceStore,
				serviceName: serviceName as any
			})
			sendUserToast(`${supportedServices[serviceName]?.displayName} disconnected successfully`)
			loadIntegrations()
		} catch (err: any) {
			sendUserToast(
				`Failed to disconnect ${supportedServices[serviceName]?.displayName}: ${err.message}`,
				true
			)
		}
	}

	async function connectService(serviceName: string, redirectUri: string) {
		if (!$workspaceStore) return

		connecting = serviceName
		try {
			const auth_url = await WorkspaceIntegrationService.generateNativeTriggerServiceConnectUrl({
				workspace: $workspaceStore,
				serviceName: serviceName as any,
				requestBody: { redirect_uri: redirectUri }
			})

			if (auth_url) {
				window.location.href = auth_url
			}
		} catch (err: any) {
			sendUserToast(
				`Failed to connect ${supportedServices[serviceName]?.displayName}: ${err.message}`,
				true
			)
			connecting = null
		}
	}

	async function createOrUpdateIntegration(serviceName: string, oauthData: any) {
		if (!$workspaceStore) return

		try {
			await WorkspaceIntegrationService.createNativeTriggerService({
				workspace: $workspaceStore,
				serviceName: serviceName as any,
				requestBody: oauthData
			})
			sendUserToast(
				`${supportedServices[serviceName]?.displayName} configuration saved successfully`
			)
			loadIntegrations()
		} catch (err: any) {
			sendUserToast(
				`Failed to configure ${supportedServices[serviceName]?.displayName}: ${err.message}`,
				true
			)
		}
	}

	function isConfigured(integration: WorkspaceIntegration): boolean {
		return (
			integration.oauth_data !== null &&
			!!integration.oauth_data.client_id &&
			!!integration.oauth_data.client_secret &&
			!!integration.oauth_data.base_url
		)
	}

	function isConnected(integration: WorkspaceIntegration): boolean {
		return isConfigured(integration) && !!(integration.oauth_data as any)?.access_token
	}

	function getIntegrationByService(serviceName: string): WorkspaceIntegration | null {
		return integrations.find((integration) => integration.service_name === serviceName) || null
	}

	async function handleOAuthCallback(
		workspace: string,
		serviceName: NativeServiceName,
		code: string,
		state: string
	) {
		console.log({ workspace, serviceName, code, state })
		processingCallback = true
		try {
			if (serviceName) {
				await WorkspaceIntegrationService.nativeTriggerServiceCallback({
					serviceName,
					workspace,
					code,
					state,
					requestBody: { redirect_uri: $page.url.toString() }
				})
				sendUserToast(`${supportedServices[serviceName]?.displayName} connected successfully!`)
				await loadIntegrations()
			} else {
				sendUserToast(`Could not handle callback missing service query args`, true)
			}
		} catch (err: any) {
			sendUserToast(`Failed to complete OAuth connection: ${err.message}`, true)
		} finally {
			const url = new URL($page.url)
			url.searchParams.delete('code')
			url.searchParams.delete('state')
			url.searchParams.delete('service')
			url.searchParams.delete('workspace')
			processingCallback = false
			goto(url.toString(), { replaceState: true, noScroll: true, keepFocus: true })
		}
	}

	$effect(() => {
		if (
			$page.url.searchParams.has('code') &&
			$page.url.searchParams.has('state') &&
			$page.url.searchParams.has('workspace') &&
			$page.url.searchParams.has('service')
		) {
			const service = $page.url.searchParams.get('service')! as NativeServiceName
			const workspace = $page.url.searchParams.get('workspace')!
			const code = $page.url.searchParams.get('code')!
			const state = $page.url.searchParams.get('state')!
			handleOAuthCallback(workspace, service, code, state)
		}
	})

	let redirectUri = $state('')

	$effect(() => {
		if ($workspaceStore) {
			loadIntegrations()
		}
	})
</script>

<div class="flex flex-col">
	<SettingsPageHeader
		title="Native Triggers (Beta)"
		description="Connect your workspace to external services for native triggers and enhanced functionality. These connections are shared across all workspace members and are required for native triggers to work."
		link="https://www.windmill.dev/docs/integrations/native-triggers"
	/>

	<Alert type="warning" title="Beta Feature">
		<p>Native Triggers is currently in beta. Nextcloud integration requires:</p>
		<ul class="list-disc pl-4 mt-2 space-y-1">
			<li>
				<a
					href="https://docs.nextcloud.com/server/latest/admin_manual/installation/source_installation.html#pretty-urls"
					target="_blank"
					rel="noopener noreferrer"
					class="underline hover:text-blue-600"
				>
					Pretty URLs
				</a>
				to be enabled on your Nextcloud instance.
			</li>
			<li
				>The Windmill integration app to be installed on your Nextcloud instance (not yet released).</li
			>
		</ul>
	</Alert>

	<div class="mt-6"></div>

	{#if processingCallback}
		<Alert type="info" title="Processing OAuth connection">
			<p class="text-sm">Completing your OAuth connection, please wait...</p>
		</Alert>
	{:else if loading}
		<div class="space-y-4">
			{#each new Array(3) as _}
				<Skeleton layout={[[6], 0.4]} />
			{/each}
		</div>
	{:else}
		<div class="space-y-4">
			{#each Object.entries(supportedServices) as [serviceName, config]}
				{@const integration = getIntegrationByService(serviceName)}
				{@const isConnecting = connecting === serviceName}
				{@const isOAuthConfigured = integration && isConfigured(integration)}
				{@const isServiceConnected = integration && isConnected(integration)}
				{@const isShowingConfig = showingConfig === serviceName}

				<div class="border border-gray-200 dark:border-gray-700 rounded-md p-4 bg-surface-tertiary">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-3">
							<div class="w-8 h-8 flex items-center justify-center">
								<config.icon class="w-6 h-6" />
							</div>
							<div class="flex flex-col">
								<div class="text-sm font-semibold text-emphasis">{config.displayName}</div>
								<div class="text-xs font-normal text-primary">{config.description}</div>
							</div>
						</div>

						<div class="flex items-center gap-2">
							{#if isServiceConnected}
								<div class="flex items-center gap-1 text-green-600 text-xs">
									<Check size={16} />
									<span class="font-semibold">Connected</span>
								</div>
								<Button
									onclick={() => connectService(serviceName, redirectUri)}
									disabled={isConnecting}
									startIcon={{ icon: Plug }}
								>
									{isConnecting ? 'Reconnecting...' : 'Reconnect'}
								</Button>
								<Button
									destructive
									onclick={() => deleteIntegration(serviceName)}
									startIcon={{ icon: X }}
								>
									Delete
								</Button>
							{:else if isOAuthConfigured}
								<Button
									variant="accent"
									onclick={() => connectService(serviceName, redirectUri)}
									disabled={isConnecting}
									startIcon={{ icon: Plug }}
								>
									{isConnecting ? 'Connecting...' : 'Connect'}
								</Button>
								<Button
									destructive
									onclick={() => deleteIntegration(serviceName)}
									startIcon={{ icon: X }}
								>
									Delete
								</Button>
							{:else}
								<Button
									variant="accent"
									onclick={() =>
										(showingConfig = showingConfig === serviceName ? null : serviceName)}
									startIcon={{ icon: Cog }}
								>
									Configure OAuth
								</Button>
							{/if}

							{#if config.docsUrl}
								<Button href={config.docsUrl} target="_blank" startIcon={{ icon: ExternalLink }}>
									Docs
								</Button>
							{/if}
						</div>
					</div>

					{#if isShowingConfig}
						<div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
							<OAuthClientConfig
								{serviceName}
								bind:redirectUri
								serviceDisplayName={config.displayName}
								existingConfig={integration?.oauth_data}
								onConfigSaved={async (oauthData) => {
									await createOrUpdateIntegration(serviceName, oauthData)
									showingConfig = null
								}}
							/>
						</div>
					{/if}
				</div>
			{/each}
		</div>

		{#if integrations.length === 0}
			<Alert type="warning" title="No Integrations Connected">
				Connect to external services above to enable native triggers for your workspace.
			</Alert>
		{/if}
	{/if}
</div>
