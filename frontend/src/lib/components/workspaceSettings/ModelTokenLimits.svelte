<script lang="ts">
	import type { AIConfig, AIProvider } from '$lib/gen'
	import { Badge, Button } from '../common'
	import { getModelMaxTokens } from '../copilot/lib'
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import SettingCard from '../instanceSettings/SettingCard.svelte'

	const MAX_TOKENS_LIMIT = 2000000

	let {
		aiProviders,
		maxTokensPerModel = $bindable()
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		maxTokensPerModel: Record<string, number>
	} = $props()

	let errors = $state<Record<string, string>>({})
	let collapsedProviders = $state<Record<string, boolean>>({})

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
			errors[modelKey] = 'Token limit must be between 1 and ' + MAX_TOKENS_LIMIT
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
		errors[modelKey] = ''
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

	function toggleProvider(provider: string) {
		collapsedProviders[provider] = !collapsedProviders[provider]
	}

	function hasCustomSettings(provider: string, models: Array<{ model: string }>): boolean {
		return models.some((m) => !isModelAtDefault(provider as AIProvider, m.model))
	}

	$effect(() => {
		// Initialize collapsedProviders to true for all providers
		collapsedProviders = {
			...Object.fromEntries(Object.keys(aiProviders).map((provider) => [provider, true]))
		}
	})
</script>

{#if Object.keys(aiProviders).length > 0}
	<SettingCard
		label="Model output limits"
		description="Configure maximum token limits for each model. These limits apply to all AI chat interactions in the workspace."
	>
		<div class="flex flex-col gap-3">
			{#each Object.entries(modelsByProvider).filter(([provider, models]) => models.length > 0) as [provider, models]}
				{@const isExpanded = !collapsedProviders[provider]}
				{@const hasCustom = hasCustomSettings(provider, models)}
				<div class="border rounded-md bg-surface-tertiary">
					<button
						type="button"
						onclick={() => toggleProvider(provider)}
						class="w-full px-4 py-1 min-h-8 flex items-center justify-between hover:bg-surface-hover transition-colors rounded-t-md"
					>
						<div class="flex items-center gap-2">
							<h4 class="font-medium text-xs capitalize">{provider}</h4>
							{#if hasCustom}
								<Badge color="blue">Modified</Badge>
							{/if}
						</div>
						{#if isExpanded}
							<ChevronUp size={16} class="text-gray-500" />
						{:else}
							<ChevronDown size={16} class="text-gray-500" />
						{/if}
					</button>

					{#if isExpanded}
						<div transition:slide|local={{ duration: 200 }} class="p-4 border-t">
							<div class="space-y-3">
								{#each models as { model }}
									{@const currentTokens = getCurrentTokensForModel(provider as AIProvider, model)}
									{@const defaultTokens = getDefaultTokensForModel(provider as AIProvider, model)}
									{@const isAtDefault = isModelAtDefault(provider as AIProvider, model)}
									<div class="flex flex-col gap-1">
										<div class="flex items-center gap-3">
											<div class="flex-1 min-w-0">
												<span class="text-xs text-primary truncate block">{model}</span>
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
											<div class="text-xs text-primary flex flex-row items-center gap-1">
												<span>Default: {defaultTokens} tokens</span>
												<Button
													variant="default"
													unifiedSize="xs"
													onclick={() => resetModelToDefault(provider as AIProvider, model)}
												>
													Reset
												</Button>
											</div>
											{#if errors[getModelKey(provider as AIProvider, model)]}
												<div class="text-xs text-red-500"
													>{errors[getModelKey(provider as AIProvider, model)]}</div
												>
											{/if}
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</SettingCard>
{/if}
