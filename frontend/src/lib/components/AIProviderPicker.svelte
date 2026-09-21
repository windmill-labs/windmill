<script lang="ts">
	import { type Snippet, untrack } from 'svelte'
	import Select from './select/Select.svelte'
	import {
		fetchAvailableModels,
		AI_PROVIDERS,
		DECISION_AI_PROVIDERS,
		aiProviderDetails
	} from './copilot/lib'
	import type { AIProvider, ProviderConfig } from '$lib/gen'
	import ResourcePicker from './ResourcePicker.svelte'
	import Toggle from './Toggle.svelte'
	import { saveConfig, removeConfig, isSameAsStoredConfig } from './aiProviderStorage'
	import AIReasoningEffortPicker from './AIReasoningEffortPicker.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		value: ProviderConfig | undefined
		disabled?: boolean
		actions?: Snippet
		/** The workspace the surface operates on, which a session or fork editor sets to something
		 *  other than the one being navigated. Resources and the models read off them are per
		 *  workspace, so without it this offers what the wrong one holds. */
		workspace?: string | undefined
		/** Offer the decision providers (TypeSafe) in place of the chat ones, for an agent step with
		 *  decision output. */
		decision?: boolean
	}

	let {
		value: _uncheckedValue = $bindable(),
		disabled = false,
		actions,
		workspace = undefined,
		decision = false
	}: Props = $props()

	let effectiveWorkspace = $derived(workspace ?? $operatingWorkspace ?? '')

	let value = $derived.by(() => {
		if (!_uncheckedValue || typeof _uncheckedValue !== 'object') return undefined
		return _uncheckedValue
	})

	let loading = $state(false)
	let availableModels = $state<string[]>([])
	let filterText = $state('')

	// Keyed by provider *and* path: two `customai` resources point at different base URLs, so they
	// do not share a model list.
	let modelsCache = new Map<string, string[]>()

	// The resource picker offers every provider type of the mode at once and the pick is what names
	// the kind.
	let offeredProviders = $derived(Object.keys(decision ? DECISION_AI_PROVIDERS : AI_PROVIDERS))
	let providerResourceTypes = $derived(offeredProviders.join(','))

	// Read once: only a field that holds nothing yet is given the mode's default kind.
	if (!_uncheckedValue) {
		_uncheckedValue = {
			kind: untrack(() => decision) ? 'typesafe' : 'openai',
			resource: '',
			model: ''
		}
	}

	// A kind the mode does not offer cannot run, and switching the output type leaves the old kind
	// behind. Its resource, model and reasoning effort belong to it, so they go with it rather than
	// surfacing only as a failed run.
	$effect(() => {
		const offered = offeredProviders
		untrack(() => {
			if (value?.kind && !offered.includes(value.kind)) {
				value.kind = decision ? 'typesafe' : 'openai'
				value.resource = ''
				value.model = ''
				value.reasoning_effort = undefined
			}
		})
	})

	let useAsDefault = $derived(isSameAsStoredConfig(value))

	// Reactive items for the Select component
	let items = $derived.by(() => {
		const r = availableModels.map((model) => ({
			label: model,
			value: model
		}))
		if (value?.model && !availableModels.find((model) => model === value.model)) {
			r.push({
				label: value.model,
				value: value.model
			})
		}
		return r
	})

	async function loadModels(signal?: AbortSignal) {
		const provider = value?.kind
		const resourceValue = value?.resource
		const resourcePath = resourceValueToPath(resourceValue)

		if (!provider || !resourcePath) {
			return
		}

		loading = true
		const cacheKey = `${effectiveWorkspace}:${provider}:${resourcePath}`
		if (modelsCache.has(cacheKey)) {
			availableModels = modelsCache.get(cacheKey) || []
			loading = false
			return
		}

		try {
			const models = await fetchAvailableModels(resourcePath, effectiveWorkspace, provider, signal)
			if (signal?.aborted) {
				return
			}
			availableModels = models
			modelsCache.set(cacheKey, models)
		} catch (e) {
			if (signal?.aborted) {
				return
			}
			// Fall back to default models for this provider
			const defaultModels = aiProviderDetails(provider).defaultModels
			availableModels = defaultModels
		} finally {
			if (!signal?.aborted) {
				loading = false
			}
		}
	}

	/**
	 * The provider kind follows the resource that was picked. Driven by the pick rather than by an
	 * effect on the picker's `valueType`, which also resolves for the value the field was opened on
	 * and would rewrite a saved config just for being looked at.
	 */
	function onResourcePicked(_path: string | undefined, type: string | undefined) {
		// An empty type is the placeholder the picker keeps for a saved path it could not find. It
		// says nothing about the provider, so the kind stands.
		if (!value || !type || !offeredProviders.includes(type)) {
			return
		}
		if (value.kind === type) {
			return
		}
		value.kind = type as AIProvider
		// Models are per provider, and a reasoning token is per model.
		value.model = ''
		value.reasoning_effort = undefined
	}

	// Helper functions to handle $res: prefix like ObjectResourceInput does
	function isResource(resourceValue: any): boolean {
		return (
			typeof resourceValue === 'string' &&
			resourceValue.length >= '$res:'.length &&
			resourceValue.startsWith('$res:')
		)
	}

	function resourceValueToPath(resourceValue: any): string | undefined {
		if (isResource(resourceValue)) {
			return resourceValue.substring('$res:'.length)
		}
		return resourceValue
	}

	function pathToResourceValue(path: string | undefined): string | undefined {
		if (path == undefined) {
			return undefined
		} else {
			return `$res:${path}`
		}
	}

	// Reload models when provider or resourcePath changes
	$effect(() => {
		const abortController = new AbortController()
		const provider = value?.kind
		const resourceValue = value?.resource
		const resourcePath = resourceValueToPath(resourceValue)

		filterText = ''

		if (provider && resourcePath) {
			loadModels(abortController.signal)
		} else {
			const defaultModels = provider ? aiProviderDetails(provider).defaultModels : []
			availableModels = defaultModels
			loading = false
		}

		return () => {
			abortController.abort()
		}
	})

	$effect(() => {
		if (useAsDefault && value && value.kind && value.resource && value.model) {
			saveConfig(value)
		}
	})
</script>

<div class="w-full flex flex-col gap-3 border rounded-md p-4">
	<div class="flex flex-col gap-1">
		<span class="text-xs font-normal text-secondary">Resource</span>
		<!-- No auto-select: this picker spans every provider type, so a single candidate means "the
		     only AI resource in the workspace" rather than "the only one of the kind this agent uses".
		     Taking it would redefine the agent's provider and drop its model, on open and unasked. -->
		<ResourcePicker
			bind:value={
				() => resourceValueToPath(value?.resource),
				(v) => {
					if (value) {
						value.resource = pathToResourceValue(v) ?? ''
					}
				}
			}
			resourceType={providerResourceTypes}
			{disabled}
			{workspace}
			placeholder="Select an AI provider resource"
			selectFirst={false}
			onValueChange={onResourcePicked}
		/>
	</div>

	<div class="flex flex-col gap-1">
		<span class="text-xs font-normal text-secondary">Model</span>
		<Select
			{items}
			bind:value={() => value?.model, (v) => value && (value.model = v ?? '')}
			placeholder="Select model"
			disabled={disabled || !value?.kind || !resourceValueToPath(value?.resource)}
			onCreateItem={(r) => {
				availableModels.push(r)
				if (value) value.model = r
			}}
			createText="Press enter to use custom model"
			{loading}
			clearable={false}
			noItemsMsg={'No models available'}
			bind:filterText
		/>
	</div>

	{#if value?.model && !decision}
		<div class="flex flex-col gap-1">
			<span class="text-xs font-normal text-secondary">Reasoning effort</span>
			<AIReasoningEffortPicker
				bind:value={() => value?.reasoning_effort, (v) => value && (value.reasoning_effort = v)}
				providerConfig={value}
				{disabled}
			/>
		</div>
	{/if}

	<!-- The default seeds new agents, which start on text output. -->
	{#if !decision}
		<div class="flex justify-end">
			<Toggle
				disabled={disabled || !value?.kind || !value?.resource || !value?.model}
				bind:checked={useAsDefault}
				options={{ right: 'Use as personal default for other new agents' }}
				size="xs"
				on:change={(e) => {
					if (!e.detail) {
						removeConfig()
					} else {
						saveConfig(value)
					}
				}}
			/>
		</div>
	{/if}

	{@render actions?.()}
</div>
