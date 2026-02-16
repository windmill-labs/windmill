<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Alert } from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { Check, X, ExternalLink, Cog, Plug } from 'lucide-svelte'
	import { NextcloudIcon } from '$lib/components/icons'
	import GoogleIcon from '$lib/components/icons/GoogleIcon.svelte'
	import { WorkspaceIntegrationService, type NativeServiceName } from '$lib/gen'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import OAuthClientConfig from './OAuthClientConfig.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import Path from '$lib/components/Path.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'

	interface WorkspaceIntegration {
		service_name: string
		resource_path?: string
		oauth_data: {
			client_id: string
			client_secret: string
			base_url: string
			instance_shared?: boolean
		} | null
	}

	interface ServiceConfig {
		name: string
		displayName: string
		description: string
		icon: any
		docsUrl?: string
		requiresBaseUrl?: boolean
		clientIdPlaceholder?: string
		clientSecretPlaceholder?: string
		setupInstructions?: string[]
	}

	const supportedServices: Record<string, ServiceConfig> = {
		nextcloud: {
			name: 'nextcloud',
			displayName: 'Nextcloud',
			description: 'Connect to Nextcloud for file operations and webhook triggers',
			icon: NextcloudIcon,
			docsUrl: 'https://www.windmill.dev/docs/integrations/nextcloud',
			setupInstructions: [
				'Create an OAuth2 application in your Nextcloud instance (Administration settings → Security → OAuth 2.0 clients)',
				'Configure the redirect URI shown below',
				'Enter the client credentials below'
			]
		},
		google: {
			name: 'google',
			displayName: 'Google',
			description: 'Connect to Google for Drive and Calendar triggers',
			icon: GoogleIcon,
			requiresBaseUrl: false,
			clientIdPlaceholder: 'xxxx.apps.googleusercontent.com',
			clientSecretPlaceholder: 'Google Cloud Console client secret',
			setupInstructions: [
				'Go to <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener" class="underline">Google Cloud Console - Credentials</a>',
				'Create an OAuth 2.0 Client ID (Web application type)',
				'Add the redirect URI shown below to "Authorized redirect URIs"',
				'Enable the <a href="https://console.cloud.google.com/apis/library/drive.googleapis.com" target="_blank" rel="noopener" class="underline">Google Drive API</a> and <a href="https://console.cloud.google.com/apis/library/calendar-json.googleapis.com" target="_blank" rel="noopener" class="underline">Google Calendar API</a> in your project',
				'Enter the client credentials below'
			]
		}
	}

	let integrations = $state<WorkspaceIntegration[]>([])
	let loading = $state(false)
	let connecting = $state<string | null>(null)
	let showingConfig = $state<string | null>(null)
	let instanceSharingAvailable = $state<Record<string, boolean>>({})
	let pendingCallback = $state<{
		serviceName: NativeServiceName
		code: string
		state: string
		workspace: string
	} | null>(null)
	let resourcePath = $state<string | undefined>(undefined)
	let pathError = $state<string | undefined>(undefined)
	let confirmationModal = createAsyncConfirmationModal()

	async function loadIntegrations() {
		if (!$workspaceStore) return

		loading = true
		try {
			const response = await WorkspaceIntegrationService.listNativeTriggerServices({
				workspace: $workspaceStore
			})
			integrations = response.map((item) => ({
				service_name: item.service_name,
				resource_path: item.resource_path ?? undefined,
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

		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		const confirmed = await confirmationModal.ask({
			title: `Disconnect ${displayName}?`,
			confirmationText: 'Disconnect',
			children: `This will delete all ${displayName} triggers associated with this integration. This action cannot be undone.`
		})
		if (!confirmed) return

		try {
			await WorkspaceIntegrationService.deleteNativeTriggerService({
				workspace: $workspaceStore,
				serviceName: serviceName as any
			})
			sendUserToast(`${displayName} disconnected successfully`)
			loadIntegrations()
		} catch (err: any) {
			sendUserToast(`Failed to disconnect ${displayName}: ${err.message}`, true)
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

	async function checkInstanceSharing() {
		if (!$workspaceStore) return

		for (const serviceName of Object.keys(supportedServices)) {
			try {
				const available =
					await WorkspaceIntegrationService.checkInstanceSharingAvailable({
						workspace: $workspaceStore,
						serviceName: serviceName as NativeServiceName
					})
				instanceSharingAvailable[serviceName] = available
			} catch {
				instanceSharingAvailable[serviceName] = false
			}
		}
	}

	async function connectWithInstanceCredentials(serviceName: string) {
		if (!$workspaceStore) return

		connecting = serviceName
		try {
			const redirectUri = getRedirectUri(serviceName)
			const auth_url = await WorkspaceIntegrationService.generateInstanceConnectUrl({
				workspace: $workspaceStore,
				serviceName: serviceName as NativeServiceName,
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

	function isConfigured(integration: WorkspaceIntegration): boolean {
		if (integration.oauth_data === null) return false
		if (integration.oauth_data.instance_shared) return true
		const serviceConfig = supportedServices[integration.service_name]
		const needsBaseUrl = serviceConfig?.requiresBaseUrl !== false
		return (
			!!integration.oauth_data.client_id &&
			!!integration.oauth_data.client_secret &&
			(!needsBaseUrl || !!integration.oauth_data.base_url)
		)
	}

	function isConnected(integration: WorkspaceIntegration): boolean {
		if (integration.oauth_data?.instance_shared) {
			return !!integration.resource_path
		}
		return isConfigured(integration) && !!integration.resource_path
	}

	function getIntegrationByService(serviceName: string): WorkspaceIntegration | null {
		return integrations.find((integration) => integration.service_name === serviceName) || null
	}

	function handleOAuthCallback(
		workspace: string,
		serviceName: NativeServiceName,
		code: string,
		state: string
	) {
		// Phase 1: store callback params and clean URL — don't call backend yet
		pendingCallback = { serviceName, code, state, workspace }

		// Pre-populate resource path from existing integration (for reconnect)
		const integration = getIntegrationByService(serviceName)
		resourcePath = integration?.resource_path ?? undefined

		const url = new URL($page.url)
		url.searchParams.delete('code')
		url.searchParams.delete('state')
		url.searchParams.delete('service')
		goto(url.toString(), { replaceState: true, noScroll: true, keepFocus: true })
	}

	async function finalizePendingCallback() {
		if (!pendingCallback) return

		const { serviceName, code, state, workspace } = pendingCallback
		try {
			const redirectUri = getRedirectUri(serviceName)
			await WorkspaceIntegrationService.nativeTriggerServiceCallback({
				serviceName,
				workspace,
				requestBody: {
					code,
					state,
					redirect_uri: redirectUri,
					resource_path: resourcePath
				}
			})
			sendUserToast(`${supportedServices[serviceName]?.displayName} connected successfully!`)
			await loadIntegrations()
		} catch (err: any) {
			sendUserToast(`Failed to complete OAuth connection: ${err.message}`, true)
		} finally {
			pendingCallback = null
			resourcePath = undefined
			pathError = undefined
		}
	}

	$effect(() => {
		if (
			$page.url.searchParams.has('code') &&
			$page.url.searchParams.has('state') &&
			$page.url.searchParams.has('service') &&
			$workspaceStore
		) {
			const service = $page.url.searchParams.get('service')! as NativeServiceName
			const code = $page.url.searchParams.get('code')!
			const state = $page.url.searchParams.get('state')!
			handleOAuthCallback($workspaceStore, service, code, state)
		}
	})

	function getRedirectUri(serviceName: string): string {
		return `${window.location.origin}/workspace_settings?tab=native_triggers&service=${serviceName}`
	}

	$effect(() => {
		if ($workspaceStore) {
			loadIntegrations()
			checkInstanceSharing()
		}
	})
</script>

<div class="flex flex-col">
	<SettingsPageHeader
		title="Native Triggers"
		description="Connect your workspace to external services for native triggers and enhanced functionality. These connections are shared across all workspace members and are required for native triggers to work."
		link="https://www.windmill.dev/docs/integrations/native-triggers"
	/>

	{#if pendingCallback}
		{@const serviceName = pendingCallback.serviceName}
		{@const config = supportedServices[serviceName]}
		<div class="border border-gray-200 dark:border-gray-700 rounded-md p-4 bg-surface-tertiary">
			<div class="text-sm font-semibold text-emphasis mb-2">
				Save {config?.displayName ?? serviceName} credentials as resource
			</div>
			<div class="text-xs text-secondary mb-3">
				Choose where to save the OAuth resource. This resource will store the access token for the
				integration.
			</div>
			<Path
				kind="resource"
				initialPath={resourcePath ?? ''}
				namePlaceholder={'native_' + serviceName}
				bind:path={resourcePath}
				bind:error={pathError}
			/>
			<div class="flex gap-2 mt-3">
				<Button
					variant="accent"
					disabled={!resourcePath || !!pathError}
					onclick={finalizePendingCallback}
				>
					Save
				</Button>
				<Button
					onclick={() => {
						pendingCallback = null
						resourcePath = undefined
						pathError = undefined
					}}
				>
					Cancel
				</Button>
			</div>
		</div>
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
									onclick={() => connectService(serviceName, getRedirectUri(serviceName))}
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
									onclick={() => connectService(serviceName, getRedirectUri(serviceName))}
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
							{:else if instanceSharingAvailable[serviceName]}
								<Button
									variant="accent"
									onclick={() => connectWithInstanceCredentials(serviceName)}
									disabled={isConnecting}
									startIcon={{ icon: Plug }}
								>
									{isConnecting ? 'Connecting...' : 'Connect'}
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

					{#if instanceSharingAvailable[serviceName] && !isOAuthConfigured}
						<div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
							<Alert type="info" title="Redirect URI required">
								<p class="text-sm mb-2">
									Your instance admin has configured Google OAuth for native triggers. Before
									connecting, ensure the following redirect URI has been added to the
									<a
										href="https://console.cloud.google.com/apis/credentials"
										target="_blank"
										rel="noopener noreferrer"
										class="underline">Google Cloud Console</a
									> by the instance admin:
								</p>
								<ClipboardPanel content={getRedirectUri(serviceName)} size="sm" />
							</Alert>
						</div>
					{/if}

					{#if isShowingConfig}
						<div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
							{#if serviceName === 'nextcloud'}
								<Alert type="info" title="Requirements" class="mb-4">
									<p>Nextcloud integration requires:</p>
									<ul class="list-disc pl-4 mt-2 space-y-1">
										<li>Nextcloud 33 or later.</li>
										<li>
											The <a
												href="https://apps.nextcloud.com/apps/integration_windmill"
												target="_blank"
												rel="noopener noreferrer"
												class="underline hover:text-blue-600"
											>
												Windmill integration app
											</a> to be installed on your Nextcloud instance.
										</li>
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
									</ul>
								</Alert>
							{/if}
							<OAuthClientConfig
								{serviceName}
								redirectUri={getRedirectUri(serviceName)}
								serviceDisplayName={config.displayName}
								existingConfig={integration?.oauth_data}
								requiresBaseUrl={config.requiresBaseUrl !== false}
								clientIdPlaceholder={config.clientIdPlaceholder}
								clientSecretPlaceholder={config.clientSecretPlaceholder}
								setupInstructions={config.setupInstructions}
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

<ConfirmationModal {...confirmationModal.props} />
