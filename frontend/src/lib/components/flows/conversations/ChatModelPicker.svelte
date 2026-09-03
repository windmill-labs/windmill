<script lang="ts">
	/**
	 * Picking a model from a chat: a resource, then one of its models.
	 *
	 * Deliberately not AIProviderPicker. That one is an authoring form rendered through
	 * SchemaForm into the flow editor and the evals scorer, so it asks for the provider
	 * separately and carries authoring-only affordances. Here the provider is whatever
	 * the chosen resource is — its `resource_type` is the provider kind — so asking
	 * again would be asking the reader to restate something already known.
	 *
	 * The reasoning effort is absent on purpose: it lives in the settings menu beside
	 * this, as a slider, exactly where the copilot's own chat puts it.
	 */
	import { ResourceService, type AIProvider } from '$lib/gen'
	import Select from '$lib/components/select/Select.svelte'
	import { AI_PROVIDERS, fetchAvailableModels } from '$lib/components/copilot/lib'
	import { resource } from 'runed'
	import { Loader2 } from 'lucide-svelte'

	type ProviderValue = {
		kind?: AIProvider
		model?: string
		resource?: string
		reasoning_effort?: string
	}

	interface Props {
		value: ProviderValue | undefined
		onChange: (value: ProviderValue) => void
		workspace: string | undefined
		disabled?: boolean
	}

	let { value, onChange, workspace, disabled = false }: Props = $props()

	const AI_RESOURCE_TYPES = Object.keys(AI_PROVIDERS)

	// `$res:` is the stored form; the pickers work in bare paths.
	const resourcePath = $derived(value?.resource?.replace(/^\$res:/, '') || undefined)

	const resources = resource(
		() => workspace,
		async (ws) => {
			if (!ws) return []
			const rows = await ResourceService.listResource({
				workspace: ws,
				resourceType: AI_RESOURCE_TYPES.join(',')
			})
			return rows.map((r) => ({
				value: r.path,
				label: r.path,
				// The row's own type is the provider; an unrecognised one is a custom endpoint.
				provider: (AI_RESOURCE_TYPES.includes(r.resource_type ?? '')
					? r.resource_type
					: 'customai') as AIProvider
			}))
		}
	)

	const provider = $derived(
		resources.current?.find((r) => r.value === resourcePath)?.provider ?? value?.kind
	)

	// Models the resource actually serves, asked of the provider. Its own catalogue is
	// the fallback, so a listing that fails or is unsupported still offers real ids
	// rather than an empty box.
	const models = resource(
		() => ({ workspace, resourcePath, provider }),
		async ({ workspace, resourcePath, provider }, _prev, { onCleanup }) => {
			if (!provider) return []
			const fallback = AI_PROVIDERS[provider]?.defaultModels ?? []
			if (!workspace || !resourcePath) return fallback
			const controller = new AbortController()
			onCleanup(() => controller.abort())
			try {
				const listed = await fetchAvailableModels(
					resourcePath,
					workspace,
					provider,
					controller.signal
				)
				return listed.length > 0 ? listed : fallback
			} catch {
				return fallback
			}
		}
	)

	function selectResource(path: string | undefined) {
		const picked = resources.current?.find((r) => r.value === path)
		onChange({
			...value,
			kind: picked?.provider,
			resource: path ? `$res:${path}` : undefined,
			// The models of one provider mean nothing to another.
			model: undefined
		})
	}
</script>

<div class="flex flex-col gap-2 min-w-0">
	<div class="flex flex-col gap-1">
		<p class="text-2xs uppercase tracking-wide text-secondary">Provider</p>
		{#if resources.loading}
			<div class="flex items-center gap-2 text-xs text-tertiary py-1">
				<Loader2 size={14} class="animate-spin" /> Loading resources...
			</div>
		{:else}
			<Select
				items={resources.current ?? []}
				value={resourcePath}
				onchange={selectResource}
				placeholder="Select an AI resource"
				{disabled}
				clearable
			/>
		{/if}
	</div>

	<div class="flex flex-col gap-1">
		<p class="text-2xs uppercase tracking-wide text-secondary">Model</p>
		<Select
			items={(models.current ?? []).map((m) => ({ value: m, label: m }))}
			value={value?.model}
			onchange={(model) => onChange({ ...value, model })}
			placeholder={provider ? 'Select a model' : 'Pick a provider first'}
			disabled={disabled || !provider}
			loading={models.loading}
			clearable
		/>
	</div>
</div>
