<script lang="ts">
	import type { AIConfig, AIProvider, ModelPriceOverride } from '$lib/gen'
	import { Badge, Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import { getKnownModelPrice, inheritedCacheRates } from '../copilot/modelPricing'
	import { modelKey } from '../copilot/modelConfig'
	import { stripLegacyThinkingSuffix } from '../copilot/reasoningRegistry'
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

	// Rates are keyed by the id the chat reports usage under, and `setCopilotInfo`
	// strips the deprecated `/thinking` suffix before the chat ever sees a model. A
	// row built from the raw config would save an override under a key nothing
	// reports, so it would sit in settings looking applied and never price anything.
	// Stripping can collapse two configured slots onto one model, hence the dedupe.
	const modelsByProvider = $derived(
		Object.entries(aiProviders).reduce(
			(acc, [provider, config]) => {
				const seen = new Set<string>()
				acc[provider] = config.models.flatMap((configured) => {
					const model = stripLegacyThinkingSuffix(configured)
					if (seen.has(model)) {
						return []
					}
					seen.add(model)
					return [{ provider: provider as AIProvider, model }]
				})
				return acc
			},
			{} as Record<string, Array<{ provider: AIProvider; model: string }>>
		)
	)

	type Field = 'input' | 'output' | 'cache_read' | 'cache_write'
	type CacheField = Extract<Field, 'cache_read' | 'cache_write'>
	type Rates = { input: number; output: number; cache_read?: number; cache_write?: number }

	const RATE_FIELDS: Field[] = ['input', 'output', 'cache_read', 'cache_write']

	// Show what a blank cache rate falls back to, so the inherited figure is visible
	// rather than implied. Read from the resolver's own helper: a placeholder that
	// computed the rule separately would drift from the price actually charged.
	function inheritedCacheRate(model: string, field: Field, rates: Rates | undefined): string {
		if (field !== 'cache_read' && field !== 'cache_write') return '—'
		const input = rates?.input
		if (input === undefined) return '—'
		const inherited = inheritedCacheRates(model, input)
		return `${+(field === 'cache_read' ? inherited.cacheRead : inherited.cacheWrite).toFixed(4)}`
	}

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

	// Emptying a cache field means "inherit again", so the key has to go: leaving the
	// old number in the override would keep charging it while the field shows the
	// inherited placeholder. Input and output are required, so an empty one is not a
	// state the override can hold; the input snaps back to the stored value on blur.
	function clearCacheRate(provider: AIProvider, model: string, field: CacheField) {
		const key = modelKey(provider, model)
		const override = modelPricing[key]
		if (!override || override[field] === undefined) {
			return
		}
		const next = { ...override }
		delete next[field]
		modelPricing = { ...modelPricing, [key]: next }
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
		description="Rates in USD per million tokens, used to cost AI chat usage. Built-in list prices are a best-effort snapshot you can adjust."
		tooltip="Rates apply only where the provider returned no cost of its own; a cost it returned is used as is. A model with no built-in price starts empty and reports usage without a cost until you set one. An empty cache rate uses the figure shown in the field, and a rate of 0 prices those tokens as free."
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
									<div class="flex flex-col gap-1">
										<div class="flex items-center gap-3 flex-wrap">
											<!-- Floor the name's width so it keeps a readable share and the rate
											     fields wrap below it, rather than the name collapsing to an
											     ellipsis on a narrow panel. -->
											<div class="flex-1 min-w-[10rem]">
												<span class="text-xs text-primary truncate block">{model}</span>
											</div>
											<div class="flex items-center gap-3 flex-wrap">
												{#each RATE_FIELDS as field}
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
																	placeholder: inheritedCacheRate(model, field, rates),
																	oninput: (e: Event & { currentTarget: HTMLInputElement }) => {
																		if (e.currentTarget.value === '') {
																			if (field === 'cache_read' || field === 'cache_write') {
																				clearCacheRate(provider as AIProvider, model, field)
																			}
																			return
																		}
																		const value = parseFloat(e.currentTarget.value)
																		if (!isNaN(value)) {
																			updateRate(provider as AIProvider, model, field, value)
																		}
																	},
																	onblur: (e: Event & { currentTarget: HTMLInputElement }) => {
																		// Resync a field the state refused, so what is shown is what is stored.
																		const stored = currentRates(provider as AIProvider, model)?.[
																			field
																		]
																		e.currentTarget.value =
																			stored === undefined ? '' : String(stored)
																		errors[key] = ''
																	}
																}}
															/>
														</div>
													</div>
												{/each}
												<span class="text-xs text-secondary whitespace-nowrap">$ / 1M</span>
											</div>
										</div>
										{#if overridden}
											<div class="text-xs text-tertiary flex flex-row items-center gap-2">
												<span>Overriding the built-in price</span>
												<button
													type="button"
													class="text-xs text-blue-500 hover:underline"
													onclick={() => resetModel(provider as AIProvider, model)}
												>
													Reset
												</button>
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
