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

	// Sentinel for "leave the provider default" — distinct from a real token.
	const MODEL_DEFAULT = '__model_default__'

	let provider = $derived(
		typeof providerConfig === 'object'
			? (providerConfig?.kind as AIProvider | undefined)
			: undefined
	)
	let model = $derived(typeof providerConfig === 'object' ? providerConfig?.model : undefined)

	let capability = $derived(
		provider && model
			? getReasoningCapability(provider, model)
			: { supported: false, levels: [], canDisable: false }
	)

	// The token that turns reasoning off on a model that reasons by default
	// (e.g. Gemini/OpenAI 'none'). Undefined means omission already disables it,
	// so "Model default" already represents off — no separate option needed.
	let offToken = $derived(provider && model ? explicitOffToken(provider, model) : undefined)

	// When the model reasons only on request (Anthropic/Bedrock), leaving the
	// effort unset already means off, so the default option is labelled "off".
	// Models that reason by default keep a distinct "Model default" plus an
	// explicit off that sends the provider's disable token (e.g. Gemini "none").
	let offViaOmission = $derived(capability.canDisable && offToken === undefined)

	// A non-empty stored token that isn't one of the suggested options (custom
	// model/level) is preserved as its own item so it stays visible and editable.
	let items = $derived.by(() => {
		const opts: { label: string; value: string }[] = [
			{ label: offViaOmission ? 'off' : 'Model default', value: MODEL_DEFAULT }
		]
		if (capability.canDisable && offToken !== undefined) {
			opts.push({ label: 'off', value: offToken })
		}
		for (const level of capability.levels) {
			opts.push({ label: level, value: level })
		}
		if (value && !opts.some((o) => o.value === value)) {
			opts.push({ label: `${value} (custom)`, value })
		}
		return opts
	})

	let selected = $derived(value ? value : MODEL_DEFAULT)

	// A model that can't reason at all would 400 if a token were sent — clear any
	// stale selection once we positively know the model is unsupported.
	$effect(() => {
		if (provider && model && !capability.supported && value !== undefined) {
			value = undefined
		}
	})

	function onSelect(v: string | undefined) {
		value = v && v !== MODEL_DEFAULT ? v : undefined
	}
</script>

{#if !provider || !model}
	<div class="text-xs text-tertiary">Select a model to configure reasoning effort.</div>
{:else if !capability.supported}
	<div class="text-xs text-tertiary">The selected model does not support reasoning effort.</div>
{:else}
	<Select
		{items}
		bind:value={() => selected, (v) => onSelect(v)}
		{disabled}
		clearable={false}
		placeholder="Model default"
	/>
{/if}
