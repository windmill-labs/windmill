<script lang="ts">
	import { WorkspaceService, type AIConfig, type AIProvider } from '$lib/gen'
	import { setCopilotInfo, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { AI_PROVIDERS, fetchAvailableModels } from '../copilot/lib'
	import TestAiKey from '../copilot/TestAIKey.svelte'
	import Description from '../Description.svelte'
	import Label from '../Label.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import ArgEnum from '../ArgEnum.svelte'
	import Button from '../common/button/Button.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { AIMode } from '../copilot/chat/AIChatManager.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import autosize from '$lib/autosize'
	import ModelTokenLimits from './ModelTokenLimits.svelte'

	const MAX_CUSTOM_PROMPT_LENGTH = 5000

	let {
		aiProviders = $bindable(),
		codeCompletionModel = $bindable(),
		defaultModel = $bindable(),
		customPrompts = $bindable(),
		maxTokensPerModel = $bindable(),
		usingOpenaiClientCredentialsOauth = $bindable()
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		codeCompletionModel: string | undefined
		defaultModel: string | undefined
		customPrompts: Record<string, string>
		maxTokensPerModel: Record<string, number>
		usingOpenaiClientCredentialsOauth: boolean
	} = $props()

	let fetchedAiModels = $state(false)
	let availableAiModels = $state(
		Object.fromEntries(
			Object.keys(AI_PROVIDERS).map((provider) => [provider, AI_PROVIDERS[provider].defaultModels])
		) as Record<AIProvider, string[]>
	)

	// Custom system prompt settings
	let selectedAiMode = $state<AIMode>(AIMode.ASK)

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
				max_tokens_per_model: Object.keys(maxTokensPerModel).length > 0 ? maxTokensPerModel : undefined
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
		sendUserToast(`Copilot settings updated`)
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
</script>

<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-lg font-semibold"> Windmill AI</div>
		<Description link="https://www.windmill.dev/docs/core_concepts/ai_generation">
			Windmill AI integrates with your favorite AI providers and models.
		</Description>
	</div>
</div>

<div class="flex flex-col gap-8">
	<div class="flex flex-col gap-2">
		<p class="font-semibold">AI Providers</p>
		<div class="flex flex-col gap-4">
			{#each Object.entries(AI_PROVIDERS) as [provider, details]}
				<div class="flex flex-col gap-2">
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
								<Tooltip class="text-blue-800 dark:text-blue-800 mt-0.5">
									Anthropic models handle tool calls better than other providers, which makes them a
									better choice for AI chat.
								</Tooltip>
							</Badge>
						{/if}
					</div>

					{#if aiProviders[provider]}
						<div class="mb-4 flex flex-col gap-2">
							<div class="flex flex-row gap-1">
								<ResourcePicker
									selectFirst
									resourceType={provider === 'openai' && usingOpenaiClientCredentialsOauth
										? 'openai_client_credentials_oauth'
										: provider}
									initialValue={aiProviders[provider].resource_path}
									bind:value={
										() => aiProviders[provider].resource_path,
										(v) => {
											aiProviders[provider].resource_path = v
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
							</Label>
							<p class="text-xs">
								If you don't see the model you want, you can type it manually in the selector.
							</p>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</div>

	{#if Object.keys(aiProviders).length > 0}
		{@const autocompleteModels = selectedAiModels.filter(
			(m) => m.startsWith('codestral-') && !m.startsWith('codestral-embed')
		)}
		<div class="flex flex-col gap-2">
			<p class="font-semibold">Settings</p>
			<div class="flex flex-col gap-4">
				<Label label="Default chat model">
					{#key Object.keys(aiProviders).length}
						<ArgEnum
							enum_={selectedAiModels}
							bind:value={defaultModel}
							disabled={false}
							autofocus={false}
							defaultValue={undefined}
							valid={true}
							create={false}
						/>
					{/key}
				</Label>

				<div class="flex flex-col gap-2">
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
							right: 'Code completion (Codestral only)',
							rightTooltip:
								'We currently only support Mistral Codestral models for code completion.'
						}}
					/>

					{#if codeCompletionModel != undefined}
						<Label label="Code completion model">
							<ArgEnum
								enum_={autocompleteModels}
								bind:value={codeCompletionModel}
								disabled={false}
								autofocus={false}
								defaultValue={undefined}
								valid={true}
								create={false}
							/>
						</Label>
					{/if}
				</div>
			</div>
		</div>
	{/if}

	{#if Object.keys(aiProviders).length > 0}
		<ModelTokenLimits {aiProviders} bind:maxTokensPerModel />
	{/if}

	{#if Object.keys(aiProviders).length > 0}
		<div class="flex flex-col gap-2">
			<p class="font-semibold">Custom system prompts</p>
			<div class="flex flex-col gap-4">
				<Label label="AI Mode">
					<ToggleButtonGroup
						bind:selected={selectedAiMode}
						on:selected={({ detail }) => {
							selectedAiMode = detail
						}}
					>
						{#snippet children({ item })}
							{#each Object.values(AIMode) as mode}
								<div class="relative">
									<ToggleButton
										value={mode}
										label={mode.charAt(0).toUpperCase() + mode.slice(1)}
										{item}
									/>
									{#if customPrompts[mode]?.length > 0}
										<div
											class="absolute -top-1 -right-1 w-2 h-2 bg-blue-500 rounded-full border border-surface"
										></div>
									{/if}
								</div>
							{/each}
						{/snippet}
					</ToggleButtonGroup>
				</Label>

				<Label
					label="Custom system prompt for {selectedAiMode.charAt(0).toUpperCase() +
						selectedAiMode.slice(1)} Mode"
				>
					<textarea
						bind:value={customPrompts[selectedAiMode]}
						placeholder="Enter a custom system prompt for {selectedAiMode} mode."
						class="w-full min-h-24 p-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary resize-y"
						rows="4"
						maxlength={MAX_CUSTOM_PROMPT_LENGTH}
						use:autosize
					></textarea>
					<div class="flex justify-end mt-1">
						<span class="text-xs text-secondary">
							{(customPrompts[selectedAiMode] ?? '').length}/{MAX_CUSTOM_PROMPT_LENGTH} characters
						</span>
					</div>
				</Label>
			</div>
		</div>
	{/if}

	<Button
		wrapperClasses="self-start"
		disabled={!Object.values(aiProviders).every((p) => p.resource_path) ||
			(codeCompletionModel != undefined && codeCompletionModel.length === 0) ||
			(Object.keys(aiProviders).length > 0 && !defaultModel)}
		on:click={editCopilotConfig}
	>
		Save
	</Button>
</div>
