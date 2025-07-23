<script lang="ts">
	import { WorkspaceService, type AIConfig, type AIProvider } from '$lib/gen'
	import { setCopilotInfo, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { AI_DEFAULT_MODELS, fetchAvailableModels } from '../copilot/lib'
	import TestAiKey from '../copilot/TestAIKey.svelte'
	import Description from '../Description.svelte'
	import Label from '../Label.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import ArgEnum from '../ArgEnum.svelte'
	import Button from '../common/button/Button.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import { StarIcon } from 'lucide-svelte'

	const aiProviderLabels: [AIProvider, string][] = [
		['openai', 'OpenAI'],
		['azure_openai', 'Azure OpenAI'],
		['anthropic', 'Anthropic'],
		['mistral', 'Mistral'],
		['deepseek', 'DeepSeek'],
		['googleai', 'Google AI'],
		['groq', 'Groq'],
		['openrouter', 'OpenRouter'],
		['togetherai', 'Together AI'],
		['customai', 'Custom AI']
	]

	let {
		aiProviders = $bindable(),
		codeCompletionModel = $bindable(),
		defaultModel = $bindable(),
		usingOpenaiClientCredentialsOauth = $bindable()
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		codeCompletionModel: string | undefined
		defaultModel: string | undefined
		usingOpenaiClientCredentialsOauth: boolean
	} = $props()

	let fetchedAiModels = $state(false)
	let availableAiModels = $state(
		Object.fromEntries(
			aiProviderLabels.map(([provider]) => [provider, AI_DEFAULT_MODELS[provider]])
		) as Record<AIProvider, string[]>
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
					availableAiModels[provider] = AI_DEFAULT_MODELS[provider]
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
			const config: AIConfig = {
				providers: aiProviders,
				code_completion_model,
				default_model
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
				availableAiModels[provider] = AI_DEFAULT_MODELS[provider]
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
			{#each aiProviderLabels as [provider, label]}
				<div class="flex flex-col gap-2">
					<Toggle
						options={{
							right: label
						}}
						checked={!!aiProviders[provider]}
						on:change={(e) => {
							if (e.detail) {
								aiProviders = {
									...aiProviders,
									[provider]: {
										resource_path: '',
										models:
											availableAiModels[provider].length > 0 ? [availableAiModels[provider][0]] : []
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
						<div class="text-xs flex flex-row gap-1 items-center text-primary mb-1">
							<StarIcon size={16} />
							Anthropic models are recommended for better performance on AI chat.
						</div>
					{/if}

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
											onAiProviderChange(provider)
										}
									}
								/>
								<TestAiKey
									aiProvider={provider}
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
