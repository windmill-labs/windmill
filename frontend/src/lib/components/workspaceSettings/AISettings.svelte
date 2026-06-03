<script lang="ts">
	import {
		AiService,
		ResourceService,
		WorkspaceService,
		type AIConfig,
		type AIProvider,
		type AIProviderModel,
		type GetCopilotSettingsStateResponse,
		type InstanceAISummary,
		type OpenAIChatGPTAccountDeviceCompleteResponse,
		type OpenAIChatGPTAccountDeviceStartResponse
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { AI_PROVIDERS, fetchAvailableModels } from '../copilot/lib'
	import { supportsAutocomplete } from '../copilot/utils'
	import TestAiKey from '../copilot/TestAIKey.svelte'
	import Label from '../Label.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import Select from '../select/Select.svelte'
	import Button from '../common/button/Button.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import Tooltip from '../Tooltip.svelte'
	import ModelTokenLimits from './ModelTokenLimits.svelte'
	import { setCopilotInfo } from '$lib/aiStore'
	import AIPromptsModal from '../settings/AIPromptsModal.svelte'
	import { Settings } from 'lucide-svelte'
	import { onDestroy, untrack } from 'svelte'
	import { slide } from 'svelte/transition'
	import SettingsFooter from './SettingsFooter.svelte'
	import InstanceFallbackSettings from './InstanceFallbackSettings.svelte'
	import SettingCard from '../instanceSettings/SettingCard.svelte'

	let {
		initialConfig = undefined,
		hasUnsavedChanges = $bindable(false),
		workspace = undefined,
		disableChatOffset = false,
		hasInstanceAiConfig = false,
		usesInstanceAiConfig = false,
		instanceAiSummary = undefined,
		customSave = undefined,
		onSave = undefined,
		title = 'Windmill AI',
		description = 'Windmill AI integrates with your favorite AI providers and models.',
		link = 'https://www.windmill.dev/docs/core_concepts/ai_generation',
		promptScope = 'workspace'
	}: {
		initialConfig?: AIConfig | undefined
		hasUnsavedChanges?: boolean
		workspace?: string | undefined
		disableChatOffset?: boolean
		hasInstanceAiConfig?: boolean
		usesInstanceAiConfig?: boolean
		instanceAiSummary?: InstanceAISummary
		customSave?: (config: AIConfig) => Promise<void>
		onSave?: (info?: GetCopilotSettingsStateResponse) => void | Promise<void>
		title?: string
		description?: string
		link?: string
		promptScope?: 'workspace' | 'instance'
	} = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	// --- Internal state ---
	let aiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let codeCompletionModelKey: string | undefined = $state(undefined)
	let defaultModelKey: string | undefined = $state(undefined)
	let customPrompts: Record<string, string> = $state({})
	let maxTokensPerModel: Record<string, number> = $state({})
	let usingOpenaiClientCredentialsOauth = $state(false)
	let workspaceOverrideEditorOpened = $state(false)
	let chatgptDeviceAuth = $state<OpenAIChatGPTAccountDeviceStartResponse | undefined>(undefined)
	let chatgptDeviceStatus = $state<string | undefined>(undefined)
	let chatgptDeviceConnecting = $state(false)
	let chatgptDevicePollTimeout: ReturnType<typeof setTimeout> | undefined = undefined

	// --- Initial state for dirty tracking ---
	let initialAiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let initialCodeCompletionModelKey: string | undefined = $state(undefined)
	let initialDefaultModelKey: string | undefined = $state(undefined)
	let initialCustomPrompts: Record<string, string> = $state({})
	let initialMaxTokensPerModel: Record<string, number> = $state({})
	let initialPrompts: Record<string, string> = $state({})
	let lastLoadedConfigKey = $state<string | undefined>(undefined)

	function clone<T>(v: T): T {
		return JSON.parse(JSON.stringify(v))
	}

	function providerLabel(provider: AIProvider): string {
		return AI_PROVIDERS[provider]?.label ?? provider
	}

	function providerModelKeyFromParts(provider: AIProvider, model: string): string {
		return `${provider}:${model}`
	}

	function providerModelKey(providerModel: AIProviderModel | undefined): string | undefined {
		return providerModel
			? providerModelKeyFromParts(providerModel.provider, providerModel.model)
			: undefined
	}

	function providerModelLabel(providerModel: AIProviderModel): string {
		return `${providerModel.model} · ${providerLabel(providerModel.provider)}`
	}

	function hasProviderModelKey(key: string | undefined): boolean {
		if (!key) return false
		return Object.entries(aiProviders).some(([provider, config]) =>
			config.models.some(
				(model) => providerModelKeyFromParts(provider as AIProvider, model) === key
			)
		)
	}

	function applyConfig(config: AIConfig | undefined) {
		aiProviders = clone(config?.providers ?? {})
		defaultModelKey = providerModelKey(config?.default_model)
		codeCompletionModelKey = providerModelKey(config?.code_completion_model)
		customPrompts = clone(config?.custom_prompts ?? {})
		maxTokensPerModel = clone(config?.max_tokens_per_model ?? {})
		for (const mode of ['edit', 'fix', 'gen']) {
			if (!(mode in customPrompts)) {
				customPrompts[mode] = ''
			}
		}
	}

	function storeInitialState() {
		initialAiProviders = clone(aiProviders)
		initialDefaultModelKey = defaultModelKey
		initialCodeCompletionModelKey = codeCompletionModelKey
		initialCustomPrompts = clone(customPrompts)
		initialMaxTokensPerModel = clone(maxTokensPerModel)
		initialPrompts = clone(customPrompts)
	}

	export function loadFromConfig(config: AIConfig | undefined) {
		applyConfig(config)
		storeInitialState()
	}

	export function discard() {
		aiProviders = clone(initialAiProviders)
		defaultModelKey = initialDefaultModelKey
		codeCompletionModelKey = initialCodeCompletionModelKey
		customPrompts = clone(initialCustomPrompts)
		maxTokensPerModel = clone(initialMaxTokensPerModel)
	}

	$effect(() => {
		const configKey = JSON.stringify(initialConfig ?? {})
		if (configKey === lastLoadedConfigKey) {
			return
		}
		lastLoadedConfigKey = configKey
		untrack(() => {
			loadFromConfig(initialConfig)
		})
	})

	// Check if openai_client_credentials_oauth resource type exists
	async function loadOpenaiOauthFlag() {
		try {
			usingOpenaiClientCredentialsOauth = await ResourceService.existsResourceType({
				workspace: effectiveWorkspace,
				path: 'openai_client_credentials_oauth'
			})
		} catch {
			usingOpenaiClientCredentialsOauth = false
		}
	}
	loadOpenaiOauthFlag()

	// --- Dirty tracking ---
	let dirty = $derived(
		JSON.stringify(aiProviders) !== JSON.stringify(initialAiProviders) ||
			defaultModelKey !== initialDefaultModelKey ||
			codeCompletionModelKey !== initialCodeCompletionModelKey ||
			JSON.stringify(customPrompts) !== JSON.stringify(initialCustomPrompts) ||
			JSON.stringify(maxTokensPerModel) !== JSON.stringify(initialMaxTokensPerModel)
	)

	$effect(() => {
		hasUnsavedChanges = dirty
	})

	// --- Model fetching ---
	let fetchedAiModels = $state(false)
	let availableAiModels = $state(
		Object.fromEntries(
			Object.keys(AI_PROVIDERS).map((provider) => [provider, AI_PROVIDERS[provider].defaultModels])
		) as Record<AIProvider, string[]>
	)

	let modalOpen = $state(false)
	let hasPromptsChanges = $derived(
		Array.from(new Set([...Object.keys(customPrompts), ...Object.keys(initialPrompts)])).some(
			(key) => {
				const currentValue = customPrompts[key] || ''
				const initialValue = initialPrompts[key] || ''
				return currentValue !== initialValue
			}
		)
	)
	let promptCount = $derived(
		Object.values(customPrompts).filter((p) => p?.trim().length > 0).length
	)
	let promptDescription = $derived(
		promptScope === 'instance'
			? 'Customize AI behavior with instance-level system prompts. These apply when a workspace uses instance AI defaults.'
			: 'Customize AI behavior with workspace-level system prompts. These apply to all workspace members.'
	)
	let showWorkspaceOverrideEditor = $derived(
		!usesInstanceAiConfig || Object.keys(aiProviders).length > 0 || workspaceOverrideEditorOpened
	)

	let selectedAiModels = $derived(
		Object.entries(aiProviders).flatMap(([provider, config]) =>
			config.models.map((model) => ({ provider: provider as AIProvider, model }))
		)
	)
	let selectedAiModelOptions = $derived(
		selectedAiModels.map((model) => ({
			value: providerModelKey(model)!,
			label: providerModelLabel(model)
		}))
	)
	let modelProviderMap = $derived(
		Object.fromEntries(
			selectedAiModels.map((model) => [providerModelKey(model)!, model])
		) as Record<string, AIProviderModel>
	)
	$effect(() => {
		if (Object.keys(aiProviders).length < 1) {
			codeCompletionModelKey = undefined
			defaultModelKey = undefined
		}
	})

	$effect(() => {
		;(async () => {
			if (fetchedAiModels) {
				return
			}
			for (const provider of Object.keys(aiProviders)) {
				try {
					const models = await fetchAvailableModels(
						aiProviders[provider].resource_path,
						effectiveWorkspace,
						provider as AIProvider
					)
					availableAiModels[provider] = models
				} catch (e) {
					console.error('failed to fetch models for provider', provider, e)
					availableAiModels[provider] =
						provider === 'openai_chatgpt_account' ? [] : AI_PROVIDERS[provider].defaultModels
				}
			}
			fetchedAiModels = true
		})()
	})

	function resetPrompts() {
		customPrompts = { ...initialPrompts }
		sendUserToast('Reset to last saved state')
	}

	function buildConfig(): AIConfig {
		const code_completion_model = codeCompletionModelKey
			? modelProviderMap[codeCompletionModelKey]
			: undefined
		const default_model = defaultModelKey ? modelProviderMap[defaultModelKey] : undefined
		const custom_prompts: Record<string, string> = Object.entries(customPrompts)
			.filter(([_, prompt]) => prompt.trim().length > 0)
			.reduce((acc, [mode, prompt]) => ({ ...acc, [mode]: prompt }), {})

		return Object.keys(aiProviders ?? {}).length > 0
			? {
					providers: aiProviders,
					code_completion_model,
					default_model,
					custom_prompts: Object.keys(custom_prompts).length > 0 ? custom_prompts : undefined,
					max_tokens_per_model:
						Object.keys(maxTokensPerModel).length > 0 ? maxTokensPerModel : undefined
				}
			: {}
	}

	function isSaveDisabled(): boolean {
		const hasInvalidDefaultModel =
			defaultModelKey != undefined &&
			(defaultModelKey.length === 0 || !modelProviderMap[defaultModelKey])
		const hasInvalidCodeCompletionModel =
			codeCompletionModelKey != undefined &&
			(codeCompletionModelKey.length === 0 || !modelProviderMap[codeCompletionModelKey])

		return (
			!Object.values(aiProviders).every((p) => p.resource_path) ||
			hasInvalidCodeCompletionModel ||
			(Object.keys(aiProviders).length > 0 && (!defaultModelKey || hasInvalidDefaultModel))
		)
	}

	export async function saveIfDirtyAndValid(): Promise<boolean> {
		if (!dirty) {
			return true
		}

		if (isSaveDisabled()) {
			sendUserToast('Complete AI settings before leaving this page', true)
			return false
		}

		await editCopilotConfig()
		return true
	}

	async function editCopilotConfig(): Promise<void> {
		const config = buildConfig()
		let settingsState: GetCopilotSettingsStateResponse | undefined

		if (customSave) {
			await customSave(config)
		} else {
			const response = await WorkspaceService.editCopilotConfig({
				workspace: effectiveWorkspace,
				requestBody: config
			})
			setCopilotInfo(response.effective_ai_config)
			settingsState = {
				has_instance_ai_config: response.has_instance_ai_config,
				uses_instance_ai_config: response.uses_instance_ai_config,
				instance_ai_summary: response.instance_ai_summary
			}
			sendUserToast('AI settings updated')
		}
		storeInitialState()
		await onSave?.(settingsState)
	}

	async function onAiProviderChange(provider: AIProvider) {
		if (aiProviders[provider].resource_path) {
			try {
				const models = await fetchAvailableModels(
					aiProviders[provider].resource_path,
					effectiveWorkspace,
					provider as AIProvider
				)
				availableAiModels[provider] = models
			} catch (e) {
				console.error('failed to fetch models for provider', provider, e)
				availableAiModels[provider] =
					provider === 'openai_chatgpt_account' ? [] : AI_PROVIDERS[provider].defaultModels
				if (provider === 'openai_chatgpt_account') {
					chatgptDeviceStatus = 'ChatGPT connected, but Codex models could not be loaded.'
				}
			}
		}

		if (
			aiProviders[provider]?.resource_path &&
			aiProviders[provider]?.models.length === 0 &&
			availableAiModels[provider].length > 0
		) {
			aiProviders[provider].models = availableAiModels[provider].slice(0, 1)
		}
	}

	function clearChatGPTDevicePoll() {
		if (chatgptDevicePollTimeout) {
			clearTimeout(chatgptDevicePollTimeout)
			chatgptDevicePollTimeout = undefined
		}
	}

	async function startChatGPTAccountDeviceAuth(provider: AIProvider) {
		clearChatGPTDevicePoll()
		chatgptDeviceConnecting = true
		chatgptDeviceAuth = undefined
		chatgptDeviceStatus = undefined

		try {
			chatgptDeviceAuth = await AiService.startOpenAiChatGptAccountDeviceAuth({
				workspace: effectiveWorkspace
			})
			chatgptDeviceStatus = 'Waiting for authorization in OpenAI...'
			void pollChatGPTAccountDeviceAuth(provider)
		} catch (error) {
			console.error('failed to start ChatGPT device auth', error)
			sendUserToast('Failed to start ChatGPT login', true)
			chatgptDeviceConnecting = false
		}
	}

	async function pollChatGPTAccountDeviceAuth(provider: AIProvider) {
		if (!chatgptDeviceAuth) {
			return
		}

		if (new Date(chatgptDeviceAuth.expires_at).getTime() <= Date.now()) {
			chatgptDeviceConnecting = false
			chatgptDeviceStatus = 'The login code expired. Start a new connection.'
			return
		}

		try {
			const response: OpenAIChatGPTAccountDeviceCompleteResponse =
				await AiService.completeOpenAiChatGptAccountDeviceAuth({
					workspace: effectiveWorkspace,
					requestBody: {
						device_auth_id: chatgptDeviceAuth.device_auth_id,
						user_code: chatgptDeviceAuth.user_code,
						resource_path: aiProviders[provider]?.resource_path || undefined
					}
				})

			if (response.status === 'connected' && response.resource_path) {
				if (!aiProviders[provider]) {
					chatgptDeviceConnecting = false
					chatgptDeviceAuth = undefined
					chatgptDeviceStatus = 'ChatGPT connected, but the provider was disabled.'
					return
				}
				aiProviders[provider].resource_path = response.resource_path
				chatgptDeviceConnecting = false
				chatgptDeviceStatus = 'ChatGPT account connected.'
				chatgptDeviceAuth = undefined
				await onAiProviderChange(provider)
				sendUserToast('ChatGPT account connected')
				return
			}

			chatgptDeviceStatus = 'Waiting for authorization in OpenAI...'
			chatgptDevicePollTimeout = setTimeout(
				() => pollChatGPTAccountDeviceAuth(provider),
				chatgptDeviceAuth.interval_ms
			)
		} catch (error) {
			console.error('failed to complete ChatGPT device auth', error)
			sendUserToast('Failed to complete ChatGPT login', true)
			chatgptDeviceConnecting = false
		}
	}

	onDestroy(clearChatGPTDevicePoll)

	const autocompleteModels = $derived(
		selectedAiModels.filter((providerModel) => supportsAutocomplete(providerModel.model))
	)
	const autocompleteModelOptions = $derived(
		autocompleteModels.map((model) => ({
			value: providerModelKey(model)!,
			label: providerModelLabel(model)
		}))
	)
</script>

<SettingsPageHeader {title} {description} {link} />

<div class="flex flex-col gap-6 mt-4 pb-8">
	{#if usesInstanceAiConfig}
		<div
			class="p-3 border border-blue-200 dark:border-blue-700 bg-blue-50 dark:bg-blue-900/20 rounded-md text-xs text-secondary"
		>
			Instance-level AI settings are currently active. Configure workspace-specific settings below
			to override them.
		</div>
		<InstanceFallbackSettings
			{instanceAiSummary}
			{showWorkspaceOverrideEditor}
			onToggleOverride={() => (workspaceOverrideEditorOpened = !workspaceOverrideEditorOpened)}
		/>
	{:else if hasInstanceAiConfig && Object.keys(aiProviders).length > 0}
		<div
			class="p-3 border border-surface-hover bg-surface-secondary rounded-md text-xs text-secondary"
		>
			Workspace AI settings override instance defaults. Remove workspace settings to use instance
			defaults.
		</div>
	{/if}
	{#if showWorkspaceOverrideEditor}
		<SettingCard label="AI Providers">
			<div class="flex flex-col gap-4 p-4 rounded-md border bg-surface-tertiary">
				{#each Object.entries(AI_PROVIDERS) as [provider, details] (provider)}
					<div class="flex flex-col">
						<div class="flex flex-row gap-2">
							<Toggle
								options={{
									right: details.label
								}}
								checked={!!aiProviders[provider]}
								on:change={(e) => {
									if (e.detail) {
										aiProviders = {
											...aiProviders,
											[provider]: {
												resource_path: '',
												models:
													availableAiModels[provider].length > 0
														? [availableAiModels[provider][0]]
														: []
											}
										}

										if (availableAiModels[provider].length > 0 && !defaultModelKey) {
											defaultModelKey = providerModelKeyFromParts(
												provider as AIProvider,
												availableAiModels[provider][0]
											)
										}
									} else {
										aiProviders = Object.fromEntries(
											Object.entries(aiProviders).filter(([key]) => key !== provider)
										)
										if (defaultModelKey && !hasProviderModelKey(defaultModelKey)) {
											defaultModelKey = undefined
										}
										if (codeCompletionModelKey && !hasProviderModelKey(codeCompletionModelKey)) {
											codeCompletionModelKey = undefined
										}
									}
								}}
							/>
							{#if provider === 'anthropic'}
								<Badge color="blue">
									Recommended
									<Tooltip>
										Anthropic models handle tool calls better than other providers, which makes them
										a better choice for AI chat.
									</Tooltip>
								</Badge>
							{/if}
						</div>

						{#if aiProviders[provider]}
							<div
								class="mb-4 flex flex-col gap-6 border p-4 rounded-md mt-2"
								transition:slide|local={{ duration: 150 }}
							>
								<Label label="Resource">
									{#if provider === 'openai_chatgpt_account'}
										<div class="mb-3 flex flex-col gap-3 rounded-md border bg-surface-secondary p-3 text-xs">
											<div class="flex flex-col gap-1">
												<span class="font-medium text-primary">Connect with ChatGPT</span>
												<span class="text-secondary">
													Use OpenAI Device Flow to create a Windmill resource backed by your
													ChatGPT account session.
												</span>
											</div>
											<Button
												onclick={() => startChatGPTAccountDeviceAuth(provider as AIProvider)}
												variant="default"
												unifiedSize="sm"
												disabled={chatgptDeviceConnecting}
											>
												{chatgptDeviceConnecting ? 'Waiting for OpenAI' : 'Connect with ChatGPT'}
											</Button>
											{#if chatgptDeviceAuth}
												<div class="flex flex-col gap-2 rounded bg-surface p-3">
													<a
														class="text-blue-600 underline"
														href={chatgptDeviceAuth.verification_uri}
														target="_blank"
														rel="noreferrer"
													>
														Open OpenAI device login
													</a>
													<div>
														Enter code <code class="rounded bg-surface-tertiary px-1 py-0.5">{chatgptDeviceAuth.user_code}</code>
													</div>
													{#if chatgptDeviceStatus}
														<div class="text-secondary">{chatgptDeviceStatus}</div>
													{/if}
												</div>
											{:else if chatgptDeviceStatus}
												<div class="text-secondary">{chatgptDeviceStatus}</div>
											{/if}
										</div>
									{/if}
									<div class="flex flex-row gap-1">
										<ResourcePicker
											selectFirst
											{workspace}
											{disableChatOffset}
											resourceType={provider === 'openai' && usingOpenaiClientCredentialsOauth
												? 'openai_client_credentials_oauth'
												: provider}
											initialValue={aiProviders[provider].resource_path}
											bind:value={
												() => aiProviders[provider].resource_path || undefined,
												(v) => {
													aiProviders[provider].resource_path = v ?? ''
													onAiProviderChange(provider as AIProvider)
												}
											}
										/>
										<TestAiKey
											aiProvider={provider as AIProvider}
											workspace={effectiveWorkspace}
											resourcePath={aiProviders[provider].resource_path}
											model={aiProviders[provider].models[0]}
										/>
									</div>
								</Label>

								<Label label="Enabled models">
									<MultiSelect
										items={safeSelectItems([
											...availableAiModels[provider],
											...aiProviders[provider].models
										])}
										bind:value={aiProviders[provider].models}
										placeholder="Select models"
										onCreateItem={(item) =>
											(aiProviders[provider].models = [...aiProviders[provider].models, item])}
									/>
									<p class="text-2xs text-hint">
										If you don't see the model you want, you can type it manually in the selector.
									</p>
								</Label>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</SettingCard>

		<SettingCard label="Default chat model">
			{#key Object.keys(aiProviders).length}
				<Select
					items={safeSelectItems(selectedAiModelOptions)}
					bind:value={defaultModelKey}
					disabled={false}
					placeholder="Select a default model"
					size="sm"
					class="max-w-lg"
				/>
			{/key}
		</SettingCard>

		<!-- Code completion group for animation purposes -->
		<div>
			<SettingCard label="Code completion">
				<Toggle
					on:change={(e) => {
						if (e.detail) {
							codeCompletionModelKey = autocompleteModelOptions[0]?.value ?? ''
						} else {
							codeCompletionModelKey = undefined
						}
					}}
					checked={codeCompletionModelKey != undefined}
					disabled={autocompleteModelOptions.length == 0}
					options={{
						right: 'Enable code completion',
						rightTooltip:
							'We currently support Mistral Codestral and DeepSeek FIM models for code completion.'
					}}
				/>
			</SettingCard>

			{#if codeCompletionModelKey != undefined}
				<div transition:slide|local={{ duration: 150 }} class="mt-6">
					<SettingCard label="Code completion model">
						<Select
							items={safeSelectItems(autocompleteModelOptions)}
							bind:value={codeCompletionModelKey}
							disabled={false}
							placeholder="Select a code completion model"
							size="sm"
						/>
					</SettingCard>
				</div>
			{/if}
		</div>

		<ModelTokenLimits {aiProviders} bind:maxTokensPerModel />

		<SettingCard label="Custom system prompts" description={promptDescription}>
			<div class="flex items-center gap-2 pt-1">
				<Button
					onclick={() => (modalOpen = true)}
					variant="default"
					unifiedSize="sm"
					startIcon={{ icon: Settings }}
					disabled={Object.keys(aiProviders ?? {}).length === 0}
				>
					Configure AI prompts
				</Button>
				{#if promptCount > 0}
					<span class="text-xs text-secondary">({promptCount} configured)</span>
				{/if}
				{#if hasPromptsChanges}
					<Badge color="yellow">Unsaved changes</Badge>
				{/if}
			</div>
		</SettingCard>
	{/if}
</div>

<AIPromptsModal
	bind:open={modalOpen}
	bind:customPrompts
	onReset={resetPrompts}
	hasChanges={hasPromptsChanges}
	scope={promptScope}
/>

{#if showWorkspaceOverrideEditor}
	<SettingsFooter
		hasUnsavedChanges={dirty}
		onSave={editCopilotConfig}
		onDiscard={discard}
		saveLabel="Save AI settings"
		disabled={isSaveDisabled()}
	/>
{/if}
