<script lang="ts">
	import { WorkspaceService, type AIConfig, type AIProvider } from '$lib/gen'
	import { setCopilotInfo, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { AI_DEFAULT_MODELS } from '../copilot/lib'
	import TestAiKey from '../copilot/TestAIKey.svelte'
	import Description from '../Description.svelte'
	import Label from '../Label.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import ArgEnum from '../ArgEnum.svelte'
	import Button from '../common/button/Button.svelte'
	import MultiSelectWrapper from '../multiselect/MultiSelectWrapper.svelte'

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

	let availableAIModels = $derived(Object.values(aiProviders).flatMap((p) => p.models))
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

	async function editCopilotConfig(): Promise<void> {
		if (Object.keys(aiProviders ?? {}).length > 0) {
			const code_completion_model = codeCompletionModel
				? { model: codeCompletionModel, provider: modelProviderMap[codeCompletionModel] }
				: undefined
			const default_model = defaultModel
				? { model: defaultModel, provider: modelProviderMap[defaultModel] }
				: undefined
			await WorkspaceService.editCopilotConfig({
				workspace: $workspaceStore!,
				requestBody: {
					providers: aiProviders,
					code_completion_model,
					default_model
				}
			})
			setCopilotInfo({
				providers: aiProviders,
				code_completion_model,
				default_model
			})
		} else {
			await WorkspaceService.editCopilotConfig({
				workspace: $workspaceStore!,
				requestBody: {}
			})
			setCopilotInfo({})
		}
		sendUserToast(`Copilot settings updated`)
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
											AI_DEFAULT_MODELS[provider].length > 0 ? [AI_DEFAULT_MODELS[provider][0]] : []
									}
								}

								if (AI_DEFAULT_MODELS[provider].length > 0 && !defaultModel) {
									defaultModel = AI_DEFAULT_MODELS[provider][0]
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
					{#if aiProviders[provider]}
						<div class="mb-4 flex flex-col gap-2">
							<div class="flex flex-row gap-1">
								{#key aiProviders[provider].resource_path}
									<!-- this can be removed once the parent component moves to runes -->
									<!-- svelte-ignore binding_property_non_reactive -->
									<ResourcePicker
										resourceType={provider === 'openai' && usingOpenaiClientCredentialsOauth
											? 'openai_client_credentials_oauth'
											: provider}
										initialValue={aiProviders[provider].resource_path}
										bind:value={aiProviders[provider].resource_path}
										on:change={() => {
											if (
												aiProviders[provider]?.resource_path &&
												aiProviders[provider]?.models.length === 0 &&
												AI_DEFAULT_MODELS[provider].length > 0
											) {
												aiProviders[provider].models = AI_DEFAULT_MODELS[provider].slice(0, 1)
											}
										}}
									/>
								{/key}
								<TestAiKey
									aiProvider={provider}
									resourcePath={aiProviders[provider].resource_path}
									model={aiProviders[provider].models[0]}
								/>
							</div>

							<Label label="Enabled models">
								<!-- this can be removed once the parent component moves to runes -->
								<!-- svelte-ignore binding_property_non_reactive -->
								<MultiSelectWrapper
									items={AI_DEFAULT_MODELS[provider]}
									bind:value={aiProviders[provider].models}
									placeholder="Select models"
									allowUserOptions="append"
								/>
							</Label>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</div>

	{#if Object.keys(aiProviders).length > 0}
		<div class="flex flex-col gap-2">
			<p class="font-semibold">Settings</p>
			<div class="flex flex-col gap-4">
				<Label label="Default chat model">
					{#key Object.keys(aiProviders).length}
						<ArgEnum
							enum_={availableAIModels}
							bind:value={defaultModel}
							disabled={false}
							autofocus={false}
							defaultValue={undefined}
							valid={true}
							create={false}
							required={false}
						/>
					{/key}
				</Label>

				<div class="flex flex-col gap-2">
					<Toggle
						on:change={(e) => {
							if (e.detail) {
								codeCompletionModel = ''
							} else {
								codeCompletionModel = undefined
							}
						}}
						checked={codeCompletionModel != undefined}
						options={{
							right: 'Code completion'
						}}
					/>

					{#if codeCompletionModel != undefined}
						<Label label="Code completion model">
							<ArgEnum
								enum_={availableAIModels}
								bind:value={codeCompletionModel}
								disabled={false}
								autofocus={false}
								defaultValue={undefined}
								valid={true}
								create={false}
								required={false}
							/>
							<p class="text-xs">
								We highly recommend using Mistral's Codestral model for code completion.
							</p>
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
