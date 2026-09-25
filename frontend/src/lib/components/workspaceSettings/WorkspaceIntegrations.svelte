<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Alert } from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { X, ExternalLink, Cog, Plug } from 'lucide-svelte'
	import { NextcloudIcon, GithubIcon } from '$lib/components/icons'
	import GoogleIcon from '$lib/components/icons/GoogleIcon.svelte'
	import {
		WorkspaceIntegrationService,
		type NativeServiceName,
		type NativeTriggerConnection
	} from '$lib/gen'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import OAuthClientConfig from './OAuthClientConfig.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import Path from '$lib/components/Path.svelte'
	import {
		connectNativeAccount,
		nativeOAuthRedirectUri
	} from '$lib/components/triggers/native/utils'

	interface WorkspaceIntegration {
		service_name: string
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
			docsUrl: 'https://www.windmill.dev/docs/core_concepts/native_triggers#google-triggers',
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
		},
		github: {
			name: 'github',
			displayName: 'GitHub',
			description: 'Connect to GitHub for repository webhook triggers',
			icon: GithubIcon,
			docsUrl: 'https://www.windmill.dev/docs/core_concepts/native_triggers#github-triggers',
			requiresBaseUrl: false,
			clientIdPlaceholder: 'GitHub OAuth App Client ID',
			clientSecretPlaceholder: 'GitHub OAuth App Client Secret',
			setupInstructions: [
				'Go to <a href="https://github.com/settings/developers" target="_blank" rel="noopener" class="underline">GitHub Developer Settings</a>',
				'Create a new OAuth App (not a GitHub App)',
				'Set the "Authorization callback URL" to the redirect URI shown below',
				'Enter the Client ID and Client Secret below'
			]
		}
	}

	let integrations = $state<WorkspaceIntegration[]>([])
	let connections = $state<Record<string, NativeTriggerConnection[]>>({})
	let loading = $state(false)
	let connecting = $state<string | null>(null)
	let showingConfig = $state<string | null>(null)
	let instanceSharingAvailable = $state<Record<string, boolean>>({})
	// The service whose "where to save the connection" form is open.
	let choosingPathFor = $state<string | null>(null)
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
				oauth_data: item.oauth_data || null
			}))
			await Promise.all(Object.keys(supportedServices).map(loadConnections))
		} catch (err: any) {
			console.error('Failed to load workspace integrations:', err)
			sendUserToast(`Failed to load integrations: ${err.message}`, true)
		} finally {
			loading = false
		}
	}

	async function loadConnections(serviceName: string) {
		if (!$workspaceStore) return
		try {
			connections[serviceName] = await WorkspaceIntegrationService.listNativeTriggerConnections({
				workspace: $workspaceStore,
				serviceName: serviceName as NativeServiceName
			})
		} catch {
			connections[serviceName] = []
		}
	}

	async function deleteIntegration(serviceName: string) {
		if (!$workspaceStore) return

		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		const confirmed = await confirmationModal.ask({
			title: `Remove the ${displayName} integration?`,
			confirmationText: 'Remove',
			children: `This removes the OAuth app configuration, every connected ${displayName} account and all ${displayName} triggers. This action cannot be undone.`
		})
		if (!confirmed) return

		try {
			await WorkspaceIntegrationService.deleteNativeTriggerService({
				workspace: $workspaceStore,
				serviceName: serviceName as NativeServiceName
			})
			sendUserToast(`${displayName} integration removed`)
			loadIntegrations()
		} catch (err: any) {
			sendUserToast(`Failed to remove ${displayName}: ${err.body ?? err.message}`, true)
		}
	}

	async function disconnect(serviceName: string, path: string) {
		if (!$workspaceStore) return

		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		const confirmed = await confirmationModal.ask({
			title: `Disconnect ${path}?`,
			confirmationText: 'Disconnect',
			children: `This deletes the ${displayName} triggers created with this account. This action cannot be undone.`
		})
		if (!confirmed) return

		try {
			await WorkspaceIntegrationService.deleteNativeTriggerConnection({
				workspace: $workspaceStore,
				serviceName: serviceName as NativeServiceName,
				path
			})
			sendUserToast(`${path} disconnected`)
			loadConnections(serviceName)
		} catch (err: any) {
			sendUserToast(`Failed to disconnect ${path}: ${err.body ?? err.message}`, true)
		}
	}

	async function connect(serviceName: string) {
		if (!$workspaceStore) return

		connecting = serviceName
		try {
			const path = await connectNativeAccount(
				$workspaceStore,
				serviceName as NativeServiceName,
				resourcePath
			)
			sendUserToast(`${supportedServices[serviceName]?.displayName} account connected as ${path}`)
			choosingPathFor = null
			await loadConnections(serviceName)
		} catch (err: any) {
			sendUserToast(
				`Failed to connect ${supportedServices[serviceName]?.displayName}: ${err.body ?? err.message}`,
				true
			)
		} finally {
			connecting = null
		}
	}

	async function createOrUpdateIntegration(serviceName: string, oauthData: any) {
		if (!$workspaceStore) return

		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		const current = getIntegrationByService(serviceName)?.oauth_data
		const clientChanges =
			current?.instance_shared || (current?.client_id ?? '') !== (oauthData.client_id ?? '')
		// A refresh token only works with the OAuth app that issued it.
		if (clientChanges && (connections[serviceName]?.length ?? 0) > 0) {
			const confirmed = await confirmationModal.ask({
				title: `Change the ${displayName} OAuth app?`,
				confirmationText: 'Change',
				children: `Accounts connected with the current app must be connected again, at the same path, before their triggers work again.`
			})
			if (!confirmed) return
		}

		try {
			await WorkspaceIntegrationService.createNativeTriggerService({
				workspace: $workspaceStore,
				serviceName: serviceName as NativeServiceName,
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
				const available = await WorkspaceIntegrationService.checkInstanceSharingAvailable({
					workspace: $workspaceStore,
					serviceName: serviceName as NativeServiceName
				})
				instanceSharingAvailable[serviceName] = available
			} catch {
				instanceSharingAvailable[serviceName] = false
			}
		}
	}

	/** Whether the workspace has its own OAuth app for the service. */
	function hasWorkspaceApp(integration: WorkspaceIntegration | null): boolean {
		if (!integration?.oauth_data || integration.oauth_data.instance_shared) return false
		const serviceConfig = supportedServices[integration.service_name]
		const needsBaseUrl = serviceConfig?.requiresBaseUrl !== false
		return (
			!!integration.oauth_data.client_id &&
			!!integration.oauth_data.client_secret &&
			(!needsBaseUrl || !!integration.oauth_data.base_url)
		)
	}

	function getIntegrationByService(serviceName: string): WorkspaceIntegration | null {
		return integrations.find((integration) => integration.service_name === serviceName) || null
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
		description="Configure the OAuth app of each service. Members then connect their own account, or one shared through a folder, and each trigger acts as the account it was created with."
		link="https://www.windmill.dev/docs/core_concepts/native_triggers"
	/>

	{#if loading}
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
				{@const workspaceApp = hasWorkspaceApp(integration)}
				{@const usesInstanceApp = !workspaceApp && instanceSharingAvailable[serviceName]}
				{@const serviceConnections = connections[serviceName] ?? []}

				<div class="border border-gray-200 dark:border-gray-700 rounded-md p-4 bg-surface-tertiary">
					<div class="flex items-center justify-between gap-4">
						<div class="flex items-center gap-3">
							<div class="w-8 h-8 flex items-center justify-center">
								<config.icon class="w-6 h-6" />
							</div>
							<div class="flex flex-col">
								<div class="text-sm font-semibold text-emphasis">{config.displayName}</div>
								<div class="text-xs font-normal text-primary">{config.description}</div>
							</div>
						</div>

						<div class="flex flex-wrap items-center justify-end gap-2">
							{#if workspaceApp || usesInstanceApp}
								<Button
									variant="default"
									onclick={() => {
										choosingPathFor = serviceName
										resourcePath = undefined
									}}
									disabled={isConnecting}
									startIcon={{ icon: Plug }}
								>
									Connect account
								</Button>
							{/if}
							<Button
								variant="default"
								onclick={() => (showingConfig = showingConfig === serviceName ? null : serviceName)}
								startIcon={{ icon: Cog }}
							>
								Configure OAuth
							</Button>
							{#if integration?.oauth_data}
								<Button
									variant="default"
									destructive
									onclick={() => deleteIntegration(serviceName)}
									startIcon={{ icon: X }}
								>
									Remove
								</Button>
							{/if}
							{#if config.docsUrl}
								<Button href={config.docsUrl} target="_blank" startIcon={{ icon: ExternalLink }}>
									Docs
								</Button>
							{/if}
						</div>
					</div>

					{#if usesInstanceApp}
						<div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
							{#if serviceName === 'google'}
								<Alert type="info" title="Uses the instance OAuth app">
									<p class="text-sm mb-2">
										Your instance admin shares a Google OAuth app. Before connecting, ensure the
										following redirect URI has been added to it in the
										<a
											href="https://console.cloud.google.com/apis/credentials"
											target="_blank"
											rel="noopener noreferrer"
											class="underline">Google Cloud Console</a
										>:
									</p>
									<ClipboardPanel content={nativeOAuthRedirectUri('google')} size="sm" />
								</Alert>
							{:else}
								<Alert type="info" title="Uses the instance OAuth app">
									Your instance admin shares a {config.displayName} OAuth app, so members can connect
									their accounts without further setup.
								</Alert>
							{/if}
						</div>
					{/if}

					{#if choosingPathFor === serviceName}
						<div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
							<div class="text-xs text-secondary mb-2">
								Choose where to save the connection. Save it in a folder to let the folder's members
								create triggers with this account.
							</div>
							<Path
								kind="resource"
								initialPath=""
								namePlaceholder={'native_' + serviceName}
								bind:path={resourcePath}
								bind:error={pathError}
							/>
							<div class="flex gap-2 mt-3">
								<Button
									variant="accent"
									disabled={!resourcePath || !!pathError || isConnecting}
									loading={isConnecting}
									onclick={() => connect(serviceName)}
								>
									Connect
								</Button>
								<Button variant="default" onclick={() => (choosingPathFor = null)}>Cancel</Button>
							</div>
						</div>
					{/if}

					{#if serviceConnections.length > 0}
						<div
							class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700 flex flex-col gap-1"
						>
							<div class="text-xs font-semibold text-emphasis mb-1">Connected accounts</div>
							{#each serviceConnections as connection (connection.path)}
								<div class="flex items-center justify-between gap-2 text-xs">
									<span class="font-mono truncate">{connection.path}</span>
									<div class="flex items-center gap-2 shrink-0">
										{#if connection.owner}
											<span class="text-secondary">connected by {connection.owner}</span>
										{/if}
										<Button
											unifiedSize="xs"
											variant="subtle"
											destructive
											onclick={() => disconnect(serviceName, connection.path)}
										>
											Disconnect
										</Button>
									</div>
								</div>
							{/each}
						</div>
					{/if}

					{#if showingConfig === serviceName}
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
								redirectUri={nativeOAuthRedirectUri(serviceName as NativeServiceName)}
								serviceDisplayName={config.displayName}
								existingConfig={workspaceApp ? integration?.oauth_data : null}
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
	{/if}
</div>

<ConfirmationModal {...confirmationModal.props} />
