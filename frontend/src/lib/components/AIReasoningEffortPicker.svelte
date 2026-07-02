<script lang="ts">
	import { Brain, ChevronDown } from 'lucide-svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import Button from './common/button/Button.svelte'
	import type { Item } from '$lib/utils'
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
			: { supported: false, levels: [], canDisable: false }
	)

	// The token that turns reasoning off on a model that reasons by default
	// (e.g. Gemini/OpenAI 'none'). Undefined means omission already disables it,
	// so leaving the effort unset is itself off.
	let offToken = $derived(provider && model ? explicitOffToken(provider, model) : undefined)

	// When the model reasons only on request (Anthropic/Bedrock), an unset effort
	// already means off, so the default choice is labelled "off". Models that
	// reason by default keep a distinct "Model default" plus an explicit off.
	let offViaOmission = $derived(capability.canDisable && offToken === undefined)

	// Label shown on the trigger for the current selection.
	let currentLabel = $derived(
		!value ? (offViaOmission ? 'off' : 'Model default') : value === offToken ? 'off' : value
	)

	let items = $derived.by((): Item[] => {
		const opts: Item[] = [
			{
				displayName: offViaOmission ? 'off' : 'Model default',
				selected: !value,
				action: () => (value = undefined)
			}
		]
		if (capability.canDisable && offToken !== undefined) {
			opts.push({
				displayName: 'off',
				selected: value === offToken,
				action: () => (value = offToken)
			})
		}
		for (const level of capability.levels) {
			opts.push({ displayName: level, selected: value === level, action: () => (value = level) })
		}
		return opts
	})

	// A model that can't reason at all would 400 if a token were sent — clear any
	// stale selection once we positively know the model is unsupported.
	// Clear a stale selection once the model no longer accepts it — either it
	// can't reason at all, or the token isn't a valid level/off for this model
	// (e.g. carrying `xhigh` from Opus onto a model that tops out at `high`).
	// Sending an unsupported token would be rejected by the provider.
	$effect(() => {
		if (!provider || !model || value === undefined) return
		const valid = capability.supported && (value === offToken || capability.levels.includes(value))
		if (!valid) value = undefined
	})
</script>

{#if !provider || !model}
	<div class="text-xs text-tertiary">Select a model to configure reasoning effort.</div>
{:else if !capability.supported}
	<div class="text-xs text-tertiary">The selected model does not support reasoning effort.</div>
{:else}
	<DropdownV2 {items} {disabled} placement="bottom-start" fixedHeight={false} class="justify-start">
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				{disabled}
				unifiedSize="xs"
				variant="default"
				startIcon={{ icon: Brain }}
				endIcon={{ icon: ChevronDown }}
				btnClasses="w-full justify-between font-normal text-secondary"
				title="Reasoning effort"
			>
				<span class="truncate">{currentLabel}</span>
			</Button>
		{/snippet}
	</DropdownV2>
{/if}
