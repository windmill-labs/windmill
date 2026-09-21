<script lang="ts">
	import type { AIConfig, AIProvider } from '$lib/gen'
	import { Badge, Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import { modelKey } from '../copilot/modelConfig'
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import SettingCard from '../instanceSettings/SettingCard.svelte'
	import { chatModelsByProvider } from './perModelSettings'

	let {
		aiProviders,
		limits = $bindable(),
		label,
		description,
		min,
		max,
		getDefault
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		/** Overrides keyed `provider:model`; a model on its default has no entry. */
		limits: Record<string, number>
		label: string
		description: string
		min: number
		max: number
		/**
		 * What a model gets without an override. An `assumed` default is only a guess for
		 * a model the built-in table does not know, so an override equal to it is kept: it
		 * records the value as checked.
		 */
		getDefault: (provider: AIProvider, model: string) => { tokens: number; assumed: boolean }
	} = $props()

	let errors = $state<Record<string, string>>({})
	let collapsedProviders = $derived<Record<string, boolean>>(
		Object.fromEntries(Object.keys(aiProviders).map((provider) => [provider, true]))
	)

	const modelsByProvider = $derived(chatModelsByProvider(aiProviders))

	function currentTokens(provider: AIProvider, model: string): number {
		return limits[modelKey(provider, model)] ?? getDefault(provider, model).tokens
	}

	function isOverridden(provider: AIProvider, model: string): boolean {
		return limits[modelKey(provider, model)] !== undefined
	}

	function updateTokens(provider: AIProvider, model: string, tokens: number) {
		const key = modelKey(provider, model)
		if (!Number.isInteger(tokens) || tokens < min || tokens > max) {
			errors[key] = `Must be a whole number between ${min} and ${max}`
			return
		}
		const fallback = getDefault(provider, model)
		if (tokens === fallback.tokens && !fallback.assumed) {
			resetModel(provider, model)
		} else {
			limits = { ...limits, [key]: tokens }
		}
		errors[key] = ''
	}

	function resetModel(provider: AIProvider, model: string) {
		const key = modelKey(provider, model)
		const next = { ...limits }
		delete next[key]
		limits = next
		errors[key] = ''
	}

	function toggleProvider(provider: string) {
		collapsedProviders = { ...collapsedProviders, [provider]: !collapsedProviders[provider] }
	}

	function hasOverrides(provider: string, models: Array<{ model: string }>): boolean {
		return models.some((m) => isOverridden(provider as AIProvider, m.model))
	}
</script>

{#if Object.keys(aiProviders).length > 0}
	<SettingCard {label} {description}>
		<div class="flex flex-col gap-3">
			{#each Object.entries(modelsByProvider).filter(([_, models]) => models.length > 0) as [provider, models] (provider)}
				{@const isExpanded = !collapsedProviders[provider]}
				<div class="border rounded-md bg-surface-tertiary">
					<Button
						variant="subtle"
						unifiedSize="sm"
						onclick={() => toggleProvider(provider)}
						wrapperClasses="w-full"
						btnClasses="w-full px-4 min-h-8 justify-between rounded-t-md rounded-b-none"
						endIcon={{ icon: isExpanded ? ChevronUp : ChevronDown }}
					>
						<div class="flex items-center gap-2">
							<h4 class="font-medium text-xs capitalize">{provider}</h4>
							{#if hasOverrides(provider, models)}
								<Badge color="blue">Modified</Badge>
							{/if}
						</div>
					</Button>

					{#if isExpanded}
						<div transition:slide|local={{ duration: 200 }} class="p-4 border-t">
							<div class="space-y-3">
								{#each models as { model } (model)}
									{@const key = modelKey(provider as AIProvider, model)}
									{@const fallback = getDefault(provider as AIProvider, model)}
									{@const overridden = isOverridden(provider as AIProvider, model)}
									<div class="flex flex-col gap-1">
										<div class="flex items-center gap-3">
											<div class="flex-1 min-w-0">
												<span class="text-xs text-primary truncate block">{model}</span>
											</div>
											<div class="flex items-center gap-2">
												<div class="w-28">
													<TextInput
														value={currentTokens(provider as AIProvider, model)}
														size="sm"
														error={!!errors[key]}
														inputProps={{
															type: 'number',
															min,
															max,
															step: 1,
															oninput: (e: Event & { currentTarget: HTMLInputElement }) => {
																const value = Number(e.currentTarget.value)
																if (e.currentTarget.value !== '' && !isNaN(value)) {
																	updateTokens(provider as AIProvider, model, value)
																}
															},
															onblur: (e: Event & { currentTarget: HTMLInputElement }) => {
																// Resync a value the state refused, so what is shown is what is stored.
																e.currentTarget.value = String(
																	currentTokens(provider as AIProvider, model)
																)
																errors[key] = ''
															}
														}}
													/>
												</div>
												<span class="text-xs text-secondary whitespace-nowrap">tokens</span>
											</div>
										</div>
										{#if overridden}
											<div class="text-xs text-tertiary flex flex-row items-center gap-2">
												<span>
													{fallback.assumed ? 'Assumed' : 'Default'}: {fallback.tokens.toLocaleString()}
													tokens
												</span>
												<Button
													variant="default"
													unifiedSize="xs"
													onclick={() => resetModel(provider as AIProvider, model)}
												>
													Reset
												</Button>
											</div>
										{/if}
										{#if errors[key]}
											<div class="text-xs text-red-500">{errors[key]}</div>
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
