<script lang="ts">
	import { type Snippet } from 'svelte'
	import Select from './select/Select.svelte'
	import { fetchAvailableModels, AI_PROVIDERS } from './copilot/lib'
	import type { AIProvider, ProviderConfig } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import ToggleButtonMore from './common/toggleButton-v2/ToggleButtonMore.svelte'
	import Toggle from './Toggle.svelte'
	import { saveConfig, removeConfig, isSameAsStoredConfig } from './aiProviderStorage'

	interface Props {
		value: ProviderConfig | undefined
		disabled?: boolean
		actions?: Snippet
	}

	let { value: _uncheckedValue = $bindable(), disabled = false, actions }: Props = $props()

	let value = $derived.by(() => {
		if (!_uncheckedValue || typeof _uncheckedValue !== 'object') return undefined
		return _uncheckedValue
	})

	let loading = $state(false)
	let availableModels = $state<string[]>([])
	let filterText = $state('')

	let modelsCache = new Map<AIProvider, string[]>()

	if (!_uncheckedValue) {
		_uncheckedValue = {
			kind: 'openai',
			resource: '',
			model: ''
		}
	}

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

	// Handle provider selection
	function onProviderChange(selectedProvider: AIProvider) {
		if (value) {
			value.kind = selectedProvider
			value.resource = ''
			value.model = ''
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
				btnText={providerOptions.findIndex((p) => p.value === value?.kind) >= 3 ? '' : 'More'}
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
							value.resource = pathToResourceValue(v) ?? ''
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
