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
	import ToggleButtonMore from './common/toggleButton-v2/ToggleButtonMore.svelte'
	import Toggle from './Toggle.svelte'

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

	let loading = $state(false)
	let availableModels = $state<string[]>([])
	let filterText = $state('')
	let useAsDefault = $state(false)

	let modelsCache = new Map<AIProvider, string[]>()

	const STORAGE_KEY = 'windmill_ai_provider_config'

	// Initialize value if undefined
	if (!value) {
		const storedConfig = loadStoredConfig()
		if (storedConfig) {
			value = storedConfig
			useAsDefault = true
		} else {
			const providers = Object.keys(AI_PROVIDERS)
			value = {
				kind: providers.length > 0 ? (providers[0] as AIProvider) : undefined,
				resource: undefined,
				model: undefined
			}
			useAsDefault = false
		}
	} else {
		useAsDefault = isSameAsStoredConfig(value)
	}

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

	// Provider options for the toggle button group
	const providerOptions = Object.entries(AI_PROVIDERS).map(([key, details]) => ({
		value: key as AIProvider,
		label: details.label
	}))

	// Check if the current config is the same as the stored config
	function isSameAsStoredConfig(config: ProviderValue): boolean {
		const storedConfig = loadStoredConfig()
		return (
			storedConfig !== undefined &&
			storedConfig?.kind === config.kind &&
			storedConfig?.resource === config.resource &&
			storedConfig?.model === config.model
		)
	}

	// Load stored configuration from localStorage
	function loadStoredConfig(): ProviderValue | undefined {
		if (typeof localStorage === 'undefined') {
			return undefined
		}
		try {
			const stored = localStorage.getItem(STORAGE_KEY)
			if (stored) {
				const parsed = JSON.parse(stored)
				// Validate that the stored provider is still available
				if (parsed.kind && AI_PROVIDERS[parsed.kind]) {
					return parsed
				}
			}
		} catch (e) {
			console.error('Failed to load AI provider config from localStorage:', e)
		}
		return undefined
	}

	// Save configuration to localStorage
	function saveConfig(config: ProviderValue) {
		if (typeof localStorage === 'undefined') {
			return
		}
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(config))
		} catch (e) {
			console.error('Failed to save AI provider config to localStorage:', e)
		}
	}

	// Remove configuration from localStorage
	function removeConfig() {
		if (typeof localStorage === 'undefined') {
			return
		}
		try {
			localStorage.removeItem(STORAGE_KEY)
		} catch (e) {
			console.error('Failed to remove AI provider config from localStorage:', e)
		}
	}

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
			const defaultModels = provider ? AI_PROVIDERS[provider]?.defaultModels || [] : []
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

<div class="w-full flex flex-col gap-1 border rounded-md p-4">
	<!-- Provider Selection -->
	<ToggleButtonGroup
		selected={value?.kind}
		onSelected={onProviderChange}
		{disabled}
		wrap
		tabListClass="w-full"
	>
		{#snippet children({ item })}
			{#each providerOptions.slice(0, 3) as option}
				<ToggleButton value={option.value} label={option.label} {item} />
			{/each}
			<ToggleButtonMore
				class="ml-auto"
				btnText={providerOptions.findIndex((p) => p.value === value.kind) >= 3 ? '' : 'More'}
				togglableItems={providerOptions.slice(3)}
				{item}
				bind:selected={() => value?.kind, (v) => v && onProviderChange(v)}
			/>
		{/snippet}
	</ToggleButtonGroup>

	<!-- Resource Selection -->
	<div class="flex flex-col rounded-md pt-2 gap-2">
		<div class="flex flex-col gap-1">
			<p class="text-xs font-normal text-primary">resource</p>
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
			<p class="text-xs font-normal text-primary">model</p>
			<Select
				{items}
				bind:value={value.model}
				placeholder="Select model"
				disabled={disabled || !value?.kind || !resourceValueToPath(value?.resource)}
				onCreateItem={(r) => {
					availableModels.push(r)
					value.model = r
				}}
				createText="Press enter to use custom model"
				{loading}
				clearable={false}
				noItemsMsg={'No models available'}
				bind:filterText
			/>
		</div>

		<!-- Use as Default Checkbox -->
		<div class="flex justify-end pt-1">
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
	</div>

	{@render actions?.()}
</div>
