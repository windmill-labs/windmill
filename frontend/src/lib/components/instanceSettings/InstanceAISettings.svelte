<script lang="ts">
	import { SettingService, WorkspaceService, type AIConfig } from '$lib/gen'
	import { setCopilotInfo } from '$lib/aiStore'
	import { workspaceStore, userStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { workspaceAIClients } from '../copilot/lib'
	import AISettings from '../workspaceSettings/AISettings.svelte'
	import { Alert, Button } from '../common'
	import SettingCard from './SettingCard.svelte'

	interface Props {
		hasUnsavedChanges?: boolean
		disableChatOffset?: boolean
		showHubSync?: boolean
	}

	let {
		hasUnsavedChanges = $bindable(false),
		disableChatOffset = false,
		showHubSync = false
	}: Props = $props()

	let initialConfig: AIConfig | undefined = $state(undefined)
	let loaded = $state(false)
	let aiSettings: AISettings | undefined = $state(undefined)

	async function loadConfig() {
		try {
			initialConfig =
				((await SettingService.getGlobal({ key: 'ai_config' })) as AIConfig | undefined) ?? {}
			loaded = true
		} catch (e) {
			console.error('Failed to load instance AI config', e)
			sendUserToast('Failed to load instance AI config', true)
		}
	}

	async function handleCustomSave(config: AIConfig) {
		const hasProviders = Object.keys(config.providers ?? {}).length > 0
		await SettingService.setGlobal({
			key: 'ai_config',
			requestBody: { value: hasProviders ? config : null }
		})
		if ($workspaceStore) {
			try {
				const effectiveConfig = await WorkspaceService.getCopilotInfo({
					workspace: $workspaceStore
				})
				setCopilotInfo(effectiveConfig)
			} catch (e) {
				console.error('Failed to refresh workspace AI state after instance save', e)
			}
		}
		sendUserToast('Instance AI settings saved')
	}

	export async function persistBeforeExit(): Promise<boolean> {
		return (await aiSettings?.saveIfDirtyAndValid()) ?? true
	}

	// Ensure stores are set (this page may bypass the (logged) layout)
	async function ensureStores() {
		if (!$workspaceStore) {
			$workspaceStore = 'admins'
		}
		if (!$userStore) {
			$userStore = await getUserExt($workspaceStore)
		}
		workspaceAIClients.init($workspaceStore)
	}

	ensureStores()
	loadConfig()

	// --- Hub sync ---
	let hubSyncStatus: 'idle' | 'loading' | 'success' | 'error' = $state('idle')
	let hubSyncMessage = $state('')

	async function syncFromHub() {
		hubSyncStatus = 'loading'
		hubSyncMessage = ''
		try {
			const res = await fetch('/api/settings/sync_cached_resource_types', { method: 'POST' })
			if (!res.ok) {
				const body = await res.text()
				throw new Error(body || res.statusText)
			}
			hubSyncMessage = await res.text()
			hubSyncStatus = 'success'
		} catch (e: any) {
			hubSyncMessage = e?.message ?? 'Failed to sync from hub'
			hubSyncStatus = 'error'
		}
	}
</script>

{#if loaded}
	{#if showHubSync}
		<SettingCard
			label="Resource types"
			description="AI providers require their resource types. Sync from the Hub if they are missing."
			class="my-4"
		>
			{#snippet headerAction()}
				<Button
					variant="default"
					unifiedSize="sm"
					loading={hubSyncStatus === 'loading'}
					onClick={syncFromHub}
				>
					Sync from hub
				</Button>
			{/snippet}
			{#if hubSyncStatus === 'success'}
				<Alert type="success" title="Resource types synced" class="mt-2">
					{hubSyncMessage}
				</Alert>
			{:else if hubSyncStatus === 'error'}
				<Alert type="error" title="Sync failed" class="mt-2">
					{hubSyncMessage}
				</Alert>
			{/if}
		</SettingCard>
	{/if}

	<AISettings
		bind:this={aiSettings}
		bind:hasUnsavedChanges
		{initialConfig}
		workspace="admins"
		{disableChatOffset}
		title="Windmill AI"
		description="Windmill AI integrates with your favorite AI providers and models. Set your AI settings at the instance level to be able to use them on all your workspaces. Workspace-level settings can override these."
		link="https://www.windmill.dev/docs/core_concepts/ai_generation"
		promptScope="instance"
		customSave={handleCustomSave}
		onSave={(savedConfig) => {
			initialConfig = savedConfig
		}}
	/>
{/if}
