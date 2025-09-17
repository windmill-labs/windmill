<script lang="ts">
	import type { AIConfig, AIProvider } from '$lib/gen'
	import { getModelMaxTokens } from '../copilot/lib'

	const MAX_TOKENS_LIMIT = 2000000

	let {
		aiProviders,
		maxTokensPerModel = $bindable()
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		maxTokensPerModel: Record<string, number>
	} = $props()

	// Group available models by provider
	const modelsByProvider = $derived(
		Object.entries(aiProviders).reduce(
			(acc, [provider, config]) => {
				acc[provider] = config.models.map((model) => ({
					provider: provider as AIProvider,
					model
				}))
				return acc
			},
			{} as Record<string, Array<{ provider: AIProvider; model: string }>>
		)
	)

	function getModelKey(provider: AIProvider, model: string): string {
		return `${provider}:${model}`
	}

	function getDefaultTokensForModel(provider: AIProvider, model: string): number {
		return getModelMaxTokens(provider, model)
	}

	function getCurrentTokensForModel(provider: AIProvider, model: string): number {
		const modelKey = getModelKey(provider, model)
		return maxTokensPerModel[modelKey] ?? getDefaultTokensForModel(provider, model)
	}

	function updateTokensForModel(provider: AIProvider, model: string, tokens: number) {
		const modelKey = getModelKey(provider, model)
		if (tokens < 1 || tokens > MAX_TOKENS_LIMIT) {
			return
		}

		const defaultTokens = getDefaultTokensForModel(provider, model)

		if (tokens === defaultTokens) {
			// Remove from object if it's the default value
			const newSettings = { ...maxTokensPerModel }
			delete newSettings[modelKey]
			maxTokensPerModel = newSettings
		} else {
			maxTokensPerModel = {
				...maxTokensPerModel,
				[modelKey]: tokens
			}
		}
	}

	function resetModelToDefault(provider: AIProvider, model: string) {
		const modelKey = getModelKey(provider, model)
		const newSettings = { ...maxTokensPerModel }
		delete newSettings[modelKey]
		maxTokensPerModel = newSettings
	}

	function isModelAtDefault(provider: AIProvider, model: string): boolean {
		const currentTokens = getCurrentTokensForModel(provider, model)
		const defaultTokens = getDefaultTokensForModel(provider, model)
		return currentTokens === defaultTokens
	}
</script>

{#if Object.keys(aiProviders).length > 0}
	<div class="flex flex-col gap-4">
		<div class="flex flex-col gap-1">
			<p class="font-semibold">Model Output Limits</p>
			<p class="text-xs text-secondary">
				Configure maximum token limits for each model. These limits apply to all AI chat
				interactions in the workspace.
			</p>
		</div>

		<div class="flex flex-col gap-4">
			{#each Object.entries(modelsByProvider) as [provider, models]}
				<div class="border border-gray-200 dark:border-gray-700 rounded-md p-4">
					<h4 class="font-medium text-sm mb-3 capitalize">{provider}</h4>
					<div class="space-y-3">
						{#each models as { model }}
							{@const currentTokens = getCurrentTokensForModel(provider as AIProvider, model)}
							{@const defaultTokens = getDefaultTokensForModel(provider as AIProvider, model)}
							{@const isAtDefault = isModelAtDefault(provider as AIProvider, model)}
							<div class="flex items-center gap-3">
								<div class="flex-1 min-w-0">
									<span class="text-sm text-primary truncate block">{model}</span>
								</div>
								<div class="flex items-center gap-2">
									<input
										type="number"
										min="1"
										max={MAX_TOKENS_LIMIT}
										value={currentTokens}
										oninput={(e) => {
											const value = parseInt(e.currentTarget.value)
											if (!isNaN(value)) {
												updateTokensForModel(provider as AIProvider, model, value)
											}
										}}
										class="w-20 px-2 py-1 text-xs text-center border border-gray-200 dark:border-gray-700 rounded bg-surface focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
									/>
									<span class="text-xs text-secondary whitespace-nowrap">tokens</span>
								</div>
							</div>
							{#if !isAtDefault}
								<div class="text-xs text-tertiary ml-0 flex flex-row items-center gap-1">
									<span>Default: {defaultTokens} tokens</span>
									<button
										type="button"
										onclick={() => resetModelToDefault(provider as AIProvider, model)}
										class="text-xs text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300 whitespace-nowrap"
										title="Reset to default ({defaultTokens})"
									>
										Reset
									</button>
								</div>
							{/if}
						{/each}
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}
