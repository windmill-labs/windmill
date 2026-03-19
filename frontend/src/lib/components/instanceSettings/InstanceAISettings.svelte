<script lang="ts">
	import { SettingService, type AIConfig } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import AISettings from '../workspaceSettings/AISettings.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'

	const AIMode = {
		Edit: 'edit',
		Fix: 'fix',
		Gen: 'gen'
	} as const

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

	let hasUnsavedChanges = $derived(
		JSON.stringify(aiProviders) !== JSON.stringify(initialAiProviders) ||
			defaultModel !== initialDefaultModel ||
			codeCompletionModel !== initialCodeCompletionModel ||
			JSON.stringify(customPrompts) !== JSON.stringify(initialCustomPrompts) ||
			JSON.stringify(maxTokensPerModel) !== JSON.stringify(initialMaxTokensPerModel)
	)

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

	loadConfig()

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
</script>

{#if loaded}
	<SettingsPageHeader
		title="Instance AI Settings"
		description="Configure default AI settings for all workspaces. Workspace-level settings will override these defaults."
		link="https://www.windmill.dev/docs/core_concepts/ai_generation"
	/>
	<p class="text-2xs text-hint mt-1 mb-4">
		Resources used here must exist in the <strong>admins</strong> workspace.
	</p>
	<AISettings
		bind:aiProviders
		bind:codeCompletionModel
		bind:defaultModel
		bind:customPrompts
		bind:maxTokensPerModel
		bind:usingOpenaiClientCredentialsOauth
		{hasUnsavedChanges}
		workspace="admins"
		customSave={handleCustomSave}
		onDiscard={discardChanges}
	/>
{/if}
