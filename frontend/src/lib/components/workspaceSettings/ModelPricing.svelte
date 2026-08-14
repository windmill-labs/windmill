<script lang="ts">
	import type { AIConfig, AIProvider, ModelPriceOverride } from '$lib/gen'
	import { Badge, Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import { getKnownModelPrice } from '../copilot/modelPricing'
	import { modelKey } from '../copilot/modelConfig'
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import SettingCard from '../instanceSettings/SettingCard.svelte'

	// A rate above this is far more likely a unit mistake (per-token instead of
	// per-million) than a real price, and a wrong rate silently inflates every
	// figure derived from it.
	const MAX_RATE = 1000

	let {
		aiProviders,
		modelPricing = $bindable()
	}: {
		aiProviders: Exclude<AIConfig['providers'], undefined>
		modelPricing: Record<string, ModelPriceOverride>
	} = $props()

	let errors = $state<Record<string, string>>({})
	let collapsedProviders = $state<Record<string, boolean>>({})

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

	type Field = 'input' | 'output' | 'cache_read' | 'cache_write'
	type Rates = { input: number; output: number; cache_read?: number; cache_write?: number }

	// A model with a built-in entry already carries its provider's cached-read
	// discount, which an override inherits, so asking for it again would be noise.
	// A model with none has no ratio to inherit — without these fields its cached
	// tokens can only be billed at the full input rate.
	const RATE_FIELDS: Field[] = ['input', 'output']
	const UNPRICED_RATE_FIELDS: Field[] = ['input', 'output', 'cache_read', 'cache_write']

	function currentRates(provider: AIProvider, model: string): Rates | undefined {
		const override = modelPricing[modelKey(provider, model)]
		if (override) {
			return {
				input: override.input,
				output: override.output,
				cache_read: override.cache_read,
				cache_write: override.cache_write
			}
		}
		const builtin = getKnownModelPrice(model)
		return builtin ? { input: builtin.input, output: builtin.output } : undefined
	}

	function isOverridden(provider: AIProvider, model: string): boolean {
		return modelPricing[modelKey(provider, model)] !== undefined
	}

	function updateRate(provider: AIProvider, model: string, field: Field, value: number) {
		const key = modelKey(provider, model)
		if (!(value >= 0) || value > MAX_RATE) {
			errors[key] = `Rate must be between 0 and ${MAX_RATE}`
			return
		}
		// An edit to either field pins both: a half-specified override would leave
		// the other rate silently tracking a built-in price the admin did not choose.
		const current = currentRates(provider, model) ?? { input: 0, output: 0 }
		modelPricing = {
			...modelPricing,
			[key]: { ...modelPricing[key], input: current.input, output: current.output, [field]: value }
		}
		errors[key] = ''
	}

	function resetModel(provider: AIProvider, model: string) {
		const key = modelKey(provider, model)
		const next = { ...modelPricing }
		delete next[key]
		modelPricing = next
		errors[key] = ''
	}

	function toggleProvider(provider: string) {
		collapsedProviders[provider] = !collapsedProviders[provider]
	}

	function hasOverrides(provider: string, models: Array<{ model: string }>): boolean {
		return models.some((m) => isOverridden(provider as AIProvider, m.model))
	}

	$effect(() => {
		collapsedProviders = {
			...Object.fromEntries(Object.keys(aiProviders).map((provider) => [provider, true]))
		}
	})
</script>

{#if Object.keys(aiProviders).length > 0}
	<SettingCard
		label="Model pricing"
		description="Rates in USD per million tokens, used to cost AI chat usage. Built-in list prices are a best-effort snapshot; set a rate here to use your negotiated one, or to price a model that has none."
	>
		<div class="flex flex-col gap-3">
			{#each Object.entries(modelsByProvider).filter(([_, models]) => models.length > 0) as [provider, models]}
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
								{#each models as { model }}
									{@const key = modelKey(provider as AIProvider, model)}
									{@const rates = currentRates(provider as AIProvider, model)}
									{@const overridden = isOverridden(provider as AIProvider, model)}
									{@const builtin = getKnownModelPrice(model)}
									<div class="flex flex-col gap-1">
										<div class="flex items-center gap-3 flex-wrap">
											<div class="flex-1 min-w-0">
												<span class="text-xs text-primary truncate block">{model}</span>
											</div>
											{#each builtin ? RATE_FIELDS : UNPRICED_RATE_FIELDS as field}
												<div class="flex items-center gap-1">
													<span class="text-xs text-secondary whitespace-nowrap">
														{field.replace('_', ' ')}
													</span>
													<div class="w-24">
														<TextInput
															value={rates?.[field] ?? ''}
															size="sm"
															error={!!errors[key]}
															inputProps={{
																type: 'number',
																min: 0,
																max: MAX_RATE,
																step: 0.01,
																placeholder: '—',
																oninput: (e: Event & { currentTarget: HTMLInputElement }) => {
																	const value = parseFloat(e.currentTarget.value)
																	if (!isNaN(value)) {
																		updateRate(provider as AIProvider, model, field, value)
																	}
																}
															}}
														/>
													</div>
												</div>
											{/each}
											<span class="text-xs text-secondary whitespace-nowrap">$ / 1M</span>
										</div>
										{#if !builtin}
											<div class="text-xs text-tertiary">
												{rates
													? 'No built-in price for this model. Cached tokens bill at the input rate unless you give them their own.'
													: 'No built-in price — usage on this model is reported without a cost until you set one.'}
											</div>
										{/if}
										{#if overridden && (rates?.input === 0 || rates?.output === 0)}
											<div class="text-xs text-red-500">
												A rate left at 0 prices those tokens as free — set both.
											</div>
										{/if}
										{#if overridden}
											<div class="text-xs text-primary flex flex-row items-center gap-1">
												<span>Overriding the built-in price</span>
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
