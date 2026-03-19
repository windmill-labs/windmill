<script lang="ts">
	import { JobService, SettingService, type AIConfig } from '$lib/gen'
	import { workspaceStore, userStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { workspaceAIClients } from '../copilot/lib'
	import AISettings from '../workspaceSettings/AISettings.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import { Alert, Button } from '../common'

	const AIMode = {
		Edit: 'edit',
		Fix: 'fix',
		Gen: 'gen'
	} as const

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

	// --- AI config state ---
	let aiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let codeCompletionModel: string | undefined = $state(undefined)
	let defaultModel: string | undefined = $state(undefined)
	let customPrompts: Record<string, string> = $state({})
	let maxTokensPerModel: Record<string, number> = $state({})
	let usingOpenaiClientCredentialsOauth = $state(false)

	let initialAiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let initialCodeCompletionModel: string | undefined = $state(undefined)
	let initialDefaultModel: string | undefined = $state(undefined)
	let initialCustomPrompts: Record<string, string> = $state({})
	let initialMaxTokensPerModel: Record<string, number> = $state({})

	let loaded = $state(false)

	let dirty = $derived(
		JSON.stringify(aiProviders) !== JSON.stringify(initialAiProviders) ||
			defaultModel !== initialDefaultModel ||
			codeCompletionModel !== initialCodeCompletionModel ||
			JSON.stringify(customPrompts) !== JSON.stringify(initialCustomPrompts) ||
			JSON.stringify(maxTokensPerModel) !== JSON.stringify(initialMaxTokensPerModel)
	)

	$effect(() => {
		hasUnsavedChanges = dirty
	})

	function clone<T>(v: T): T {
		return JSON.parse(JSON.stringify(v))
	}

	function storeInitialState() {
		initialAiProviders = clone(aiProviders)
		initialDefaultModel = defaultModel
		initialCodeCompletionModel = codeCompletionModel
		initialCustomPrompts = clone(customPrompts)
		initialMaxTokensPerModel = clone(maxTokensPerModel)
	}

	async function loadConfig() {
		try {
			const config = (await SettingService.getGlobal({ key: 'ai_config' })) as
				| AIConfig
				| undefined
			aiProviders = config?.providers ?? {}
			defaultModel = config?.default_model?.model
			codeCompletionModel = config?.code_completion_model?.model
			customPrompts = config?.custom_prompts ?? {}
			maxTokensPerModel = config?.max_tokens_per_model ?? {}
			for (const mode of Object.values(AIMode)) {
				if (!(mode in customPrompts)) {
					customPrompts[mode] = ''
				}
			}
			storeInitialState()
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
		storeInitialState()
	}

	function discardChanges() {
		aiProviders = clone(initialAiProviders)
		defaultModel = initialDefaultModel
		codeCompletionModel = initialCodeCompletionModel
		customPrompts = clone(initialCustomPrompts)
		maxTokensPerModel = clone(initialMaxTokensPerModel)
	}

	// --- Ensure stores are set (this page may bypass the (logged) layout) ---
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
	<SettingsPageHeader
		title="AI Configuration"
		description="Configure default AI providers for all workspaces. Workspace-level settings can override these."
		link="https://www.windmill.dev/docs/core_concepts/ai_generation"
	/>

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

	<p class="text-2xs text-hint mb-2">
		Resources used here must exist in the <strong>admins</strong> workspace.
	</p>

	<AISettings
		bind:aiProviders
		bind:codeCompletionModel
		bind:defaultModel
		bind:customPrompts
		bind:maxTokensPerModel
		bind:usingOpenaiClientCredentialsOauth
		hasUnsavedChanges={dirty}
		workspace="admins"
		{disableChatOffset}
		customSave={handleCustomSave}
		onSave={storeInitialState}
		onDiscard={discardChanges}
	/>
{/if}
