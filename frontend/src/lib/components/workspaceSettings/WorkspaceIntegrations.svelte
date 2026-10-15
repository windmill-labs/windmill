<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, Alert } from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import NextcloudIcon from '$lib/components/icons/NextcloudIcon.svelte'
	import GithubIcon from '$lib/components/icons/GithubIcon.svelte'
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
	import NativeConnectionPicker from '$lib/components/triggers/native/NativeConnectionPicker.svelte'
	import { nativeOAuthRedirectUri } from '$lib/components/triggers/native/utils'

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
	// The service whose OAuth app dialog is open.
	let configOpen = $state<Record<string, boolean>>(
		Object.fromEntries(Object.keys(supportedServices).map((s) => [s, false]))
	)
	let instanceSharingAvailable = $state<Record<string, boolean>>({})
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

	async function setWorkspaceConnection(serviceName: string, path: string | undefined) {
		if (!$workspaceStore) return

		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		if (path) {
			const owner = connections[serviceName]?.find((c) => c.path === path)?.owner
			const account = owner ? `this account, connected by ${owner}` : 'this account'
			const confirmed = await confirmationModal.ask({
				title: `Share ${path} with the workspace?`,
				confirmationText: 'Share',
				children: `Every member will be able to create and edit ${displayName} triggers that act as ${account}. They will not see its token. It replaces any connection shared before.`
			})
			if (!confirmed) return
		} else {
			const confirmed = await confirmationModal.ask({
				title: 'Stop sharing the workspace connection?',
				confirmationText: 'Stop sharing',
				children: `Members will no longer be able to create or edit ${displayName} triggers through it, unless they can read it. Its triggers keep running.`
			})
			if (!confirmed) return
		}

		try {
			await WorkspaceIntegrationService.setNativeTriggerWorkspaceConnection({
				workspace: $workspaceStore,
				serviceName: serviceName as NativeServiceName,
				requestBody: { path: path ?? null }
			})
			sendUserToast(path ? `${path} is now the workspace connection` : 'Stopped sharing')
			await loadConnections(serviceName)
		} catch (err: any) {
			sendUserToast(`Failed to update the workspace connection: ${err.body ?? err.message}`, true)
		}
	}

	async function openConfig(serviceName: string, leavingInstanceApp: boolean) {
		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		// A refresh token only works with the OAuth app that issued it.
		if (leavingInstanceApp && (connections[serviceName]?.length ?? 0) > 0) {
			const confirmed = await confirmationModal.ask({
				title: `Use your own ${displayName} OAuth app?`,
				confirmationText: 'Continue',
				children: `Accounts connected with the instance app will have to be connected again, at the same path, before their triggers work again.`
			})
			if (!confirmed) return
		}
		configOpen[serviceName] = true
	}

	async function createOrUpdateIntegration(serviceName: string, oauthData: any) {
		if (!$workspaceStore) return

		const displayName = supportedServices[serviceName]?.displayName ?? serviceName
		const current = getIntegrationByService(serviceName)?.oauth_data
		// Leaving the instance app is confirmed when the form opens; this covers editing our own app.
		const clientChanges =
			!!current?.client_id &&
			!current.instance_shared &&
			current.client_id !== (oauthData.client_id ?? '')
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
		description="Configure the OAuth app of each service. Members connect their own account when creating a trigger, or use the workspace connection, and each trigger acts as the account it was created with."
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
				{@const workspaceApp = hasWorkspaceApp(integration)}
				{@const usesInstanceApp = !workspaceApp && instanceSharingAvailable[serviceName]}
				{@const workspaceConnection = connections[serviceName]?.find((c) => c.workspace)?.path}

				<div class="border border-light rounded-md p-4 bg-surface-tertiary flex flex-col gap-4">
					<div class="flex items-center justify-between gap-4">
						<div class="flex items-center gap-3">
							<config.icon class="w-6 h-6 shrink-0" />
							<div class="flex flex-col">
								<div class="text-sm font-semibold text-emphasis">{config.displayName}</div>
								<div class="text-xs text-secondary">{config.description}</div>
							</div>
						</div>
						{#if config.docsUrl}
							<Button
								unifiedSize="sm"
								variant="subtle"
								href={config.docsUrl}
								target="_blank"
								startIcon={{ icon: ExternalLink }}
							>
								Docs
							</Button>
						{/if}
					</div>

					<div class="border-t border-light pt-4 flex flex-col gap-4">
						<div class="flex items-center justify-between gap-4">
							<div class="flex flex-col gap-1">
								<div class="text-xs font-semibold text-emphasis">
									{#if workspaceApp}
										Workspace OAuth app
									{:else if usesInstanceApp}
										Shared by your instance admin
									{:else}
										No OAuth app yet
									{/if}
								</div>
								{#if usesInstanceApp}
									<button
										class="text-xs font-normal text-accent hover:underline w-fit"
										onclick={() => openConfig(serviceName, true)}
									>
										Use a workspace OAuth app instead
									</button>
								{/if}
							</div>
							<div class="flex items-center gap-2 shrink-0">
								{#if workspaceApp}
									<Button
										unifiedSize="sm"
										variant="default"
										onclick={() => openConfig(serviceName, false)}
									>
										Edit
									</Button>
									<Button
										unifiedSize="sm"
										variant="default"
										destructive
										onclick={() => deleteIntegration(serviceName)}
									>
										Remove
									</Button>
								{:else if !usesInstanceApp}
									<Button
										unifiedSize="sm"
										variant="default"
										onclick={() => openConfig(serviceName, false)}
									>
										Set up OAuth app
									</Button>
								{/if}
							</div>
						</div>

						{#if usesInstanceApp && serviceName === 'google'}
							<div class="flex flex-col gap-1">
								<div class="text-2xs text-hint">
									The instance app must list this redirect URI in the
									<a
										href="https://console.cloud.google.com/apis/credentials"
										target="_blank"
										rel="noopener noreferrer"
										class="underline">Google Cloud Console</a
									>
								</div>
								<ClipboardPanel content={nativeOAuthRedirectUri('google')} size="sm" />
							</div>
						{/if}

						{#if workspaceApp || usesInstanceApp}
							<div class="flex flex-col gap-1">
								<div class="text-xs font-semibold text-emphasis">Workspace connection</div>
								<div class="text-xs text-secondary">
									Members can create triggers as this account without seeing its token.
								</div>
								<NativeConnectionPicker
									service={serviceName as NativeServiceName}
									value={workspaceConnection}
									autoSelect={false}
									clearable
									labelWorkspace={false}
									placeholder="No workspace connection"
									onSelect={(path) => setWorkspaceConnection(serviceName, path)}
									onConnectionsChange={() => loadConnections(serviceName)}
								/>
							</div>
						{/if}
					</div>

					<OAuthClientConfig
						bind:open={configOpen[serviceName]}
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
							configOpen[serviceName] = false
						}}
					>
						{#snippet requirements()}
							{#if serviceName === 'nextcloud'}
								<Alert type="info" title="Nextcloud requirements">
									<ul class="list-disc pl-4 flex flex-col gap-1">
										<li>Nextcloud 33 or later.</li>
										<li>
											The <a
												href="https://apps.nextcloud.com/apps/integration_windmill"
												target="_blank"
												rel="noopener noreferrer"
												class="underline">Windmill integration app</a
											> installed.
										</li>
										<li>
											<a
												href="https://docs.nextcloud.com/server/latest/admin_manual/installation/source_installation.html#pretty-urls"
												target="_blank"
												rel="noopener noreferrer"
												class="underline">Pretty URLs</a
											> enabled.
										</li>
									</ul>
								</Alert>
							{/if}
						{/snippet}
					</OAuthClientConfig>
				</div>
			{/each}
		</div>
	{/if}
</div>

<ConfirmationModal {...confirmationModal.props} />
