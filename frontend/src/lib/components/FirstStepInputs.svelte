<script lang="ts">
	import { createEventDispatcher, onDestroy, getContext } from 'svelte'
	import { getFirstStepSchema } from '$lib/components/flows/flowStore'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { twMerge } from 'tailwind-merge'
	import { Alert } from '$lib/components/common'
	import FlowModuleSchemaItemViewer from '$lib/components/flows/map/FlowModuleSchemaItemViewer.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { prettyLanguage } from '$lib/common'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import { Building } from 'lucide-svelte'

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	let schema: Record<string, any> | undefined = $state(undefined)

	let error: string | undefined = $state(undefined)
	let mod: any | undefined = $state(undefined)
	async function loadSchema() {
		try {
			const res = await getFirstStepSchema($flowStateStore, flowStore)
			schema = res.schema
			mod = res.mod
			dispatch('connectFirstNode', { connectFirstNode: res.connectFirstNode })
		} catch (e) {
			error = e
		}
	}
	$effect(() => {
		$flowStore && $flowStateStore && loadSchema()
	})

	function handleClick() {
		selected = !selected
		dispatch('select', selected ? schema : undefined)
	}

	onDestroy(() => {
		selected = false
		dispatch('select', undefined)
	})

	let selected: boolean = $state(false)

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selected) {
			selected = false
			dispatch('select', undefined)
			event.stopPropagation()
			event.preventDefault()
		}
	}

	let firstUpdate = $state(true)
	$effect(() => {
		if (schema && !selected && firstUpdate) {
			firstUpdate = false
			setTimeout(() => {
				selected = true
				dispatch('select', schema)
			}, 200)
		}
	})

	export function resetSelected(dispatchEvent?: boolean) {
		selected = false
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-full">
	{#if schema && mod}
		<button
			class={twMerge(
				'w-full shadow-md rounded-sm',
				selected ? 'bg-surface-selected' : 'hover:bg-surface-hover'
			)}
			disabled={!schema}
			onclick={handleClick}
		>
			<FlowModuleSchemaItemViewer
				onclick={handleClick}
				deletable={false}
				id={mod.id}
				label={mod.summary ||
					(`path` in mod.value ? mod.value.path : undefined) ||
					(mod.value.type === 'rawscript'
						? `Inline ${prettyLanguage(mod.value.language)}`
						: 'To be defined')}
				path={`path` in mod.value && mod.summary ? mod.value.path : ''}
			>
				{#snippet icon()}
					<div>
						{#if mod.value.type === 'rawscript'}
							<LanguageIcon lang={mod.value.language} width={16} height={16} />
						{:else if mod.value.type === 'script'}
							{#if mod.value.path.startsWith('hub/')}
								<div>
									<IconedResourceType
										width="20px"
										height="20px"
										name={mod.value.path.split('/')[2]}
										silent={true}
									/>
								</div>
							{:else}
								<Building size={14} />
							{/if}
						{/if}
					</div>
				{/snippet}
			</FlowModuleSchemaItemViewer>
		</button>
	{:else}
		<Alert type="warning" title="Cannot load first step's inputs" size="xs">
			{error}
		</Alert>
	{/if}
</div>
