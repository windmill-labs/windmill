<script lang="ts">
	import { JobService, SettingService, type AIConfig } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { workspaceAIClients } from '../copilot/lib'
	import AISettings from '../workspaceSettings/AISettings.svelte'
	import { Alert, Button } from '../common'

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
			initialConfig = ((await SettingService.getGlobal({ key: 'ai_config' })) as
				| AIConfig
				| undefined) ?? {}
			loaded = true
		} catch (e) {
			console.error('Failed to load instance AI config', e)
			sendUserToast('Failed to load instance AI config', true)
		}
	}

	async function handleCustomSave(config: AIConfig) {
		await SettingService.setGlobal({
			key: 'ai_config',
			requestBody: { value: config }
		})
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
			await JobService.runWaitResultScriptByPath({
				workspace: 'admins',
				path: 'u/admin/hub_sync',
				requestBody: {}
			})
			hubSyncStatus = 'success'
			hubSyncMessage = 'Resource types synced from hub successfully'
		} catch (e: any) {
			hubSyncMessage =
				e?.body?.error?.message ||
				e?.body?.message ||
				(typeof e?.body === 'string' ? e.body : null) ||
				e?.message ||
				'Failed to sync from hub'
			hubSyncStatus = 'error'
		}
	}
</script>

{#if loaded}
	{#if showHubSync}
		<div
			class="p-3 border rounded-md bg-surface-secondary mb-4 mt-4 flex items-center justify-between gap-4"
		>
			<div>
				<p class="text-xs font-medium text-secondary">Resource types</p>
				<p class="text-2xs text-tertiary mt-0.5">
					AI providers require their resource types. Sync from the Hub if they are missing.
				</p>
			</div>
			<Button
				variant="default"
				unifiedSize="sm"
				loading={hubSyncStatus === 'loading'}
				onClick={syncFromHub}
			>
				Sync from hub
			</Button>
		</div>
		{#if hubSyncStatus === 'success'}
			<div class="mb-4">
				<Alert type="success" title="Resource types synced">
					{hubSyncMessage}
				</Alert>
			</div>
		{:else if hubSyncStatus === 'error'}
			<div class="mb-4">
				<Alert type="error" title="Sync failed">
					{hubSyncMessage}
				</Alert>
			</div>
		{/if}
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
		customSave={handleCustomSave}
	/>
{/if}
