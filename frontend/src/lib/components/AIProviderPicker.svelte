<script lang="ts">
	import { type Snippet } from 'svelte'
	import Select from './select/Select.svelte'
	import { fetchAvailableModels, AI_PROVIDERS } from './copilot/lib'
	import type { AIProvider } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ResourcePicker from './ResourcePicker.svelte'

	interface ProviderValue {
		kind?: AIProvider
		resource?: string
		model?: string
	}

	interface Props {
		value: ProviderValue | undefined
		disabled?: boolean
		actions?: Snippet
	}

	let { value = $bindable(), disabled = false, actions }: Props = $props()

	// Initialize value if undefined
	if (!value) {
		value = {}
	}

	let loading = $state(false)
	let availableModels = $state<string[]>([])
	let filterText = $state('')

	let modelsCache = new Map<AIProvider, string[]>()

	// Reactive items for the Select component
	let items = $derived(
		availableModels.map((model) => ({
			label: model,
			value: model
		}))
	)

	// Provider options for the toggle button group
	const providerOptions = Object.entries(AI_PROVIDERS).map(([key, details]) => ({
		value: key as AIProvider,
		label: details.label
	}))

	async function loadModels(signal?: AbortSignal) {
		const provider = value?.kind
		const resourceValue = value?.resource
		const resourcePath = resourceValueToPath(resourceValue)

		if (!provider || !resourcePath) {
			return
		}

		loading = true
		if (modelsCache.has(provider)) {
			availableModels = modelsCache.get(provider) || []
			loading = false
			return
		}

		try {
			const workspace = get(workspaceStore) || ''
			const models = await fetchAvailableModels(resourcePath, workspace, provider, signal)
			if (signal?.aborted) {
				return
			}
			availableModels = models
			modelsCache.set(provider, models)
		} catch (e) {
			if (signal?.aborted) {
				return
			}
			// Fall back to default models for this provider
			const defaultModels = AI_PROVIDERS[provider]?.defaultModels || []
			availableModels = defaultModels
		} finally {
			if (!signal?.aborted) {
				loading = false
			}
		}
	}

	// Reload models when provider or resourcePath changes
	$effect(() => {
		const abortController = new AbortController()
		const provider = value?.kind
		const resourceValue = value?.resource
		const resourcePath = resourceValueToPath(resourceValue)

		filterText = ''
		if (value) {
			value.model = undefined
		}

		if (provider && resourcePath) {
			loadModels(abortController.signal)
		} else {
			const defaultModels = provider ? AI_PROVIDERS[provider]?.defaultModels || [] : []
			availableModels = defaultModels
			loading = false
		}

		return () => {
			abortController.abort()
		}
	})

	// Handle provider selection
	function onProviderChange(selectedProvider: AIProvider) {
		if (value) {
			value.kind = selectedProvider
			value.resource = undefined
			value.model = undefined
		}
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
</script>

<div class="w-full flex flex-col gap-3">
	<!-- Provider Selection -->
	<div class="flex flex-col gap-2">
		<ToggleButtonGroup selected={value?.kind} onSelected={onProviderChange} {disabled}>
			{#snippet children({ item })}
				{#each providerOptions as option}
					<ToggleButton value={option.value} label={option.label} {item} />
				{/each}
			{/snippet}
		</ToggleButtonGroup>
	</div>

	<!-- Resource Selection -->
	<div class="flex flex-col border border-gray-200 rounded-md p-2 gap-2">
		<div class="flex flex-col gap-1">
			<p class="text-sm font-medium text-primary">resource</p>
			<ResourcePicker
				bind:value={
					() => resourceValueToPath(value?.resource),
					(v) => {
						if (value) {
							value.resource = pathToResourceValue(v)
						}
					}
				}
				resourceType={value?.kind}
				disabled={disabled || !value?.kind}
				placeholder="Select resource"
				selectFirst={true}
			/>
		</div>

		<!-- Model Selection -->
		<div class="flex flex-col gap-1">
			<p class="text-sm font-medium text-primary">model</p>
			<Select
				{items}
				bind:value={value.model}
				placeholder="Select model"
				disabled={disabled || !value?.kind || !resourceValueToPath(value?.resource)}
				{loading}
				clearable={false}
				noItemsMsg={'No models available'}
				bind:filterText
			/>
		</div>
	</div>

	{@render actions?.()}
</div>
