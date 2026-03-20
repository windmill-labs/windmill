<script lang="ts">
	import {
		ResourceService,
		WorkspaceService,
		type AIConfig,
		type AIProvider,
		type GetCopilotInfoResponse
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
	import { slide } from 'svelte/transition'
	import SettingsFooter from './SettingsFooter.svelte'
	import SettingCard from '../instanceSettings/SettingCard.svelte'

	let {
		initialConfig = undefined,
		hasUnsavedChanges = $bindable(false),
		workspace = undefined,
		disableChatOffset = false,
		hasInstanceAiConfig = false,
		usesInstanceAiConfig = false,
		customSave = undefined,
		onSave = undefined,
		title = 'Windmill AI',
		description = 'Windmill AI integrates with your favorite AI providers and models.',
		link = 'https://www.windmill.dev/docs/core_concepts/ai_generation'
	}: {
		initialConfig?: AIConfig | undefined
		hasUnsavedChanges?: boolean
		workspace?: string | undefined
		disableChatOffset?: boolean
		hasInstanceAiConfig?: boolean
		usesInstanceAiConfig?: boolean
		customSave?: (config: AIConfig) => Promise<void>
		onSave?: (info?: GetCopilotInfoResponse) => void | Promise<void>
		title?: string
		description?: string
		link?: string
	} = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	// --- Internal state ---
	let aiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let codeCompletionModel: string | undefined = $state(undefined)
	let defaultModel: string | undefined = $state(undefined)
	let customPrompts: Record<string, string> = $state({})
	let maxTokensPerModel: Record<string, number> = $state({})
	let usingOpenaiClientCredentialsOauth = $state(false)

	// --- Initial state for dirty tracking ---
	let initialAiProviders: Exclude<AIConfig['providers'], undefined> = $state({})
	let initialCodeCompletionModel: string | undefined = $state(undefined)
	let initialDefaultModel: string | undefined = $state(undefined)
	let initialCustomPrompts: Record<string, string> = $state({})
	let initialMaxTokensPerModel: Record<string, number> = $state({})

	function clone<T>(v: T): T {
		return JSON.parse(JSON.stringify(v))
	}

	function applyConfig(config: AIConfig | undefined) {
		aiProviders = config?.providers ?? {}
		defaultModel = config?.default_model?.model
		codeCompletionModel = config?.code_completion_model?.model
		customPrompts = config?.custom_prompts ?? {}
		maxTokensPerModel = config?.max_tokens_per_model ?? {}
		for (const mode of ['edit', 'fix', 'gen']) {
			if (!(mode in customPrompts)) {
				customPrompts[mode] = ''
			}
		}
	}

	function storeInitialState() {
		initialAiProviders = clone(aiProviders)
		initialDefaultModel = defaultModel
		initialCodeCompletionModel = codeCompletionModel
		initialCustomPrompts = clone(customPrompts)
		initialMaxTokensPerModel = clone(maxTokensPerModel)
	}

	export function loadFromConfig(config: AIConfig | undefined) {
		applyConfig(config)
		storeInitialState()
	}

	export function discard() {
		aiProviders = clone(initialAiProviders)
		defaultModel = initialDefaultModel
		codeCompletionModel = initialCodeCompletionModel
		customPrompts = clone(initialCustomPrompts)
		maxTokensPerModel = clone(initialMaxTokensPerModel)
	}

	// Initialize from prop
	if (initialConfig) {
		applyConfig(initialConfig)
		storeInitialState()
	}

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
			defaultModel !== initialDefaultModel ||
			codeCompletionModel !== initialCodeCompletionModel ||
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
	let initialPrompts = $state($state.snapshot(customPrompts))
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

	let selectedAiModels = $derived(Object.values(aiProviders).flatMap((p) => p.models))
	let modelProviderMap = $derived(
		Object.fromEntries(
			Object.entries(aiProviders).flatMap(([provider, config]) =>
				config.models.map((m) => [m, provider as AIProvider])
			)
		)
	)
	$effect(() => {
		if (Object.keys(aiProviders).length < 1) {
			codeCompletionModel = undefined
			defaultModel = undefined
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
					availableAiModels[provider] = AI_PROVIDERS[provider].defaultModels
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
		const code_completion_model =
			codeCompletionModel && modelProviderMap[codeCompletionModel]
				? { model: codeCompletionModel, provider: modelProviderMap[codeCompletionModel] }
				: undefined
		const default_model =
			defaultModel && modelProviderMap[defaultModel]
				? { model: defaultModel, provider: modelProviderMap[defaultModel] }
				: undefined
		const custom_prompts: Record<string, string> = Object.entries(customPrompts)
			.filter(([_, prompt]) => prompt.trim().length > 0)
			.reduce((acc, [mode, prompt]) => ({ ...acc, [mode]: prompt }), {})

		return Object.keys(aiProviders ?? {}).length > 0
			? {
					providers: aiProviders,
					code_completion_model,
					default_model,
					custom_prompts:
						Object.keys(custom_prompts).length > 0 ? custom_prompts : undefined,
					max_tokens_per_model:
						Object.keys(maxTokensPerModel).length > 0 ? maxTokensPerModel : undefined
				}
			: {}
	}

	function isSaveDisabled(): boolean {
		return (
			!Object.values(aiProviders).every((p) => p.resource_path) ||
			(codeCompletionModel != undefined && codeCompletionModel.length === 0) ||
			(Object.keys(aiProviders).length > 0 && !defaultModel)
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
		let effectiveConfig: GetCopilotInfoResponse | undefined

		if (customSave) {
			await customSave(config)
		} else {
			await WorkspaceService.editCopilotConfig({
				workspace: effectiveWorkspace,
				requestBody: config
			})
			effectiveConfig = await WorkspaceService.getCopilotInfo({
				workspace: effectiveWorkspace
			})
			setCopilotInfo(effectiveConfig)
			sendUserToast('AI settings updated')
		}
		initialPrompts = { ...customPrompts }
		storeInitialState()
		await onSave?.(effectiveConfig)
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
				availableAiModels[provider] = AI_PROVIDERS[provider].defaultModels
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

	const autocompleteModels = $derived(selectedAiModels.filter(supportsAutocomplete))
</script>

<SettingsPageHeader
	{title}
	{description}
	{link}
/>

<div class="flex flex-col gap-6 mt-4 pb-8">
	{#if usesInstanceAiConfig}
		<div class="p-3 border border-blue-200 dark:border-blue-700 bg-blue-50 dark:bg-blue-900/20 rounded-md text-xs text-secondary">
			Instance-level AI settings are currently active. Configure workspace-specific settings below to override them.
		</div>
	{:else if hasInstanceAiConfig && Object.keys(aiProviders).length > 0}
		<div class="p-3 border border-surface-hover bg-surface-secondary rounded-md text-xs text-secondary">
			Workspace AI settings override instance defaults. Remove workspace settings to use instance defaults.
		</div>
	{/if}
	<SettingCard label="AI Providers">
		<div class="flex flex-col gap-4 p-4 rounded-md border bg-surface-tertiary">
			{#each Object.entries(AI_PROVIDERS) as [provider, details]}
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

									if (availableAiModels[provider].length > 0 && !defaultModel) {
										defaultModel = availableAiModels[provider][0]
									}
								} else {
									aiProviders = Object.fromEntries(
										Object.entries(aiProviders).filter(([key]) => key !== provider)
									)
									if (defaultModel) {
										const currentDefaultModel = Object.values(aiProviders).find(
											(p) => defaultModel && p.models.includes(defaultModel)
										)
										if (!currentDefaultModel) {
											defaultModel = undefined
										}
									}
									if (codeCompletionModel) {
										const currentCodeCompletionModel = Object.values(aiProviders).find(
											(p) => codeCompletionModel && p.models.includes(codeCompletionModel)
										)
										if (!currentCodeCompletionModel) {
											codeCompletionModel = undefined
										}
									}
								}
							}}
						/>
						{#if provider === 'anthropic'}
							<Badge color="blue">
								Recommended
								<Tooltip>
									Anthropic models handle tool calls better than other providers, which makes them a
									better choice for AI chat.
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
				items={safeSelectItems(selectedAiModels)}
				bind:value={defaultModel}
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
						codeCompletionModel = autocompleteModels[0] ?? ''
					} else {
						codeCompletionModel = undefined
					}
				}}
				checked={codeCompletionModel != undefined}
				disabled={autocompleteModels.length == 0}
				options={{
					right: 'Enable code completion',
					rightTooltip: 'We currently only support Mistral Codestral models for code completion.'
				}}
			/>
		</SettingCard>

		{#if codeCompletionModel != undefined}
			<div transition:slide|local={{ duration: 150 }} class="mt-6">
				<SettingCard label="Code completion model">
					<Select
						items={safeSelectItems(autocompleteModels)}
						bind:value={codeCompletionModel}
						disabled={false}
						placeholder="Select a code completion model"
						size="sm"
					/>
				</SettingCard>
			</div>
		{/if}
	</div>

	<ModelTokenLimits {aiProviders} bind:maxTokensPerModel />

	<SettingCard
		label="Custom system prompts"
		description="Customize AI behavior with workspace-level system prompts. These apply to all workspace
			members."
	>
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
</div>

<AIPromptsModal
	bind:open={modalOpen}
	bind:customPrompts
	onReset={resetPrompts}
	hasChanges={hasPromptsChanges}
	isWorkspaceSettings={true}
/>

<SettingsFooter
	hasUnsavedChanges={dirty}
	onSave={editCopilotConfig}
	onDiscard={discard}
	saveLabel="Save AI settings"
	disabled={isSaveDisabled()}
/>
