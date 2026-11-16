<script lang="ts">
	import Select from './select/Select.svelte'
	import type { AIProvider } from '$lib/gen'
	import { getReasoningCapability, explicitOffToken } from './copilot/reasoningRegistry'

	interface Props {
		// The provider-native reasoning token sent to the model (e.g. `high`,
		// `none`), or undefined to leave the provider default untouched.
		value: string | undefined
		// The selected provider config; only `kind` and `model` are read.
		providerConfig: { kind?: AIProvider; model?: string } | string | undefined
		disabled?: boolean
	}

	let { value = $bindable(), providerConfig, disabled = false }: Props = $props()

	let provider = $derived(
		typeof providerConfig === 'object'
			? (providerConfig?.kind as AIProvider | undefined)
			: undefined
	)
	let model = $derived(typeof providerConfig === 'object' ? providerConfig?.model : undefined)

	let capability = $derived(
		provider && model
			? getReasoningCapability(provider, model)
			: { supported: false, levels: [], canDisable: false, known: false }
	)

	// The token that turns reasoning off on a model that reasons by default
	// (e.g. Gemini/OpenAI 'none'). Undefined means omission already disables it,
	// so leaving the effort unset is itself off.
	let offToken = $derived(provider && model ? explicitOffToken(provider, model) : undefined)

	// When the model reasons only on request (Anthropic/Bedrock), an unset effort
	// already means off, so the default choice is labelled "off". Models that
	// reason by default keep a distinct "Model default" plus an explicit off.
	let offViaOmission = $derived(capability.canDisable && offToken === undefined)

	// Select cannot show an item for an undefined value, so the unset effort is
	// carried by this sentinel and mapped back to undefined on write.
	const UNSET = '__model_default__'

	let items = $derived.by(() => {
		const opts = [{ label: offViaOmission ? 'off' : 'Model default', value: UNSET }]
		if (capability.canDisable && offToken !== undefined) {
			opts.push({ label: 'off', value: offToken })
		}
		for (const level of capability.levels) {
			opts.push({ label: level, value: level })
		}
		return opts
	})

	// Clear a stale selection once the model no longer accepts it — either it
	// can't reason at all, or the token isn't a valid level/off for this model
	// (e.g. carrying `xhigh` from Opus onto a model that tops out at `high`).
	// Sending an unsupported token would be rejected by the provider.
	$effect(() => {
		if (!provider || !model || value === undefined) return
		// Valid values are exactly the selectable options: the levels, plus the
		// explicit off token only when this model can actually disable reasoning.
		const valid =
			capability.supported &&
			(capability.levels.includes(value) ||
				(capability.canDisable && offToken !== undefined && value === offToken))
		if (!valid) value = undefined
	})
</script>

{#if !provider || !model}
	<div class="text-xs text-tertiary">Select a model to configure reasoning effort.</div>
{:else if !capability.supported}
	<div class="text-xs text-tertiary">The selected model does not support reasoning effort.</div>
{:else}
	<Select
		{items}
		bind:value={() => value ?? UNSET, (v) => (value = !v || v === UNSET ? undefined : v)}
		{disabled}
		clearable={false}
	/>
{/if}
