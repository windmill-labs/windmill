<script lang="ts">
	import { WorkspaceService, type AIConfig, type AIProvider } from '$lib/gen'
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
		aiProviders = $bindable(),
		codeCompletionModel = $bindable(),
		defaultModel = $bindable(),
		customPrompts = $bindable(),
		maxTokensPerModel = $bindable(),
		usingOpenaiClientCredentialsOauth = $bindable(),
		onSave,
		onDiscard,
		hasUnsavedChanges = false
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		codeCompletionModel: string | undefined
		defaultModel: string | undefined
		customPrompts: Record<string, string>
		maxTokensPerModel: Record<string, number>
		usingOpenaiClientCredentialsOauth: boolean
		onSave?: () => void
		onDiscard?: () => void
		hasUnsavedChanges?: boolean
	} = $props()

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
						$workspaceStore!,
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

	async function editCopilotConfig(): Promise<void> {
		if (Object.keys(aiProviders ?? {}).length > 0) {
			const code_completion_model =
				codeCompletionModel && modelProviderMap[codeCompletionModel]
					? { model: codeCompletionModel, provider: modelProviderMap[codeCompletionModel] }
					: undefined
			const default_model =
				defaultModel && modelProviderMap[defaultModel]
					? { model: defaultModel, provider: modelProviderMap[defaultModel] }
					: undefined
			// Convert customPrompts to include only non-empty prompts
			const custom_prompts: Record<string, string> = Object.entries(customPrompts)
				.filter(([_, prompt]) => prompt.trim().length > 0)
				.reduce((acc, [mode, prompt]) => ({ ...acc, [mode]: prompt }), {})

			const config: AIConfig = {
				providers: aiProviders,
				code_completion_model,
				default_model,
				custom_prompts: Object.keys(custom_prompts).length > 0 ? custom_prompts : undefined,
				max_tokens_per_model:
					Object.keys(maxTokensPerModel).length > 0 ? maxTokensPerModel : undefined
			}
			await WorkspaceService.editCopilotConfig({
				workspace: $workspaceStore!,
				requestBody: config
			})
			setCopilotInfo(config)
		} else {
			await WorkspaceService.editCopilotConfig({
				workspace: $workspaceStore!,
				requestBody: {}
			})
			setCopilotInfo({})
		}
		sendUserToast(`AI settings updated`)
		initialPrompts = { ...customPrompts } // Update initial prompts after successful save
		onSave?.()
	}

	async function onAiProviderChange(provider: AIProvider) {
		if (aiProviders[provider].resource_path) {
			try {
				const models = await fetchAvailableModels(
					aiProviders[provider].resource_path,
					$workspaceStore!,
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
	title="Windmill AI"
	description="Windmill AI integrates with your favorite AI providers and models."
	link="https://www.windmill.dev/docs/core_concepts/ai_generation"
/>

<div class="flex flex-col gap-6 mt-4 pb-8">
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
	{hasUnsavedChanges}
	onSave={editCopilotConfig}
	onDiscard={() => onDiscard?.()}
	saveLabel="Save AI settings"
	disabled={!Object.values(aiProviders).every((p) => p.resource_path) ||
		(codeCompletionModel != undefined && codeCompletionModel.length === 0) ||
		(Object.keys(aiProviders).length > 0 && !defaultModel)}
/>
