<script lang="ts">
	import { onMount, type Snippet } from 'svelte'
	import Select from './select/Select.svelte'
	import { fetchAvailableModels, AI_PROVIDERS } from './copilot/lib'
	import type { AIProvider } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { get } from 'svelte/store'

	interface Props {
		value: string | undefined
		provider: AIProvider | undefined
		resourcePath: string | undefined
		disabled?: boolean
		placeholder?: string
		actions?: Snippet
	}

	let {
		value = $bindable(),
		provider,
		resourcePath,
		disabled = false,
		placeholder = 'Select model',
		actions
	}: Props = $props()

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

	async function loadModels() {
		filterText = ''
		value = undefined
		if (!provider || !resourcePath) {
			// Use default models if we don't have provider/resource info
			const defaultModels = provider ? AI_PROVIDERS[provider]?.defaultModels || [] : []
			availableModels = defaultModels
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
			const models = await fetchAvailableModels(resourcePath, workspace, provider)
			availableModels = models
			modelsCache.set(provider, models)
		} catch (e) {
			// Fall back to default models for this provider
			const defaultModels = AI_PROVIDERS[provider]?.defaultModels || []
			availableModels = defaultModels
		} finally {
			loading = false
		}
	}

	onMount(() => {
		loadModels()
	})

	// Reload models when provider or resourcePath changes
	$effect(() => {
		if (provider || resourcePath) {
			loadModels()
		}
	})

	$effect(() => {
		if (provider) {
		}
	})
</script>

<div class="w-full">
	<Select
		{items}
		bind:value
		{placeholder}
		{disabled}
		{loading}
		clearable={false}
		noItemsMsg={'No models available'}
		bind:filterText
	/>
	{@render actions?.()}
</div>
