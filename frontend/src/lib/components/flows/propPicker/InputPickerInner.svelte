<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext, onMount } from 'svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { twMerge } from 'tailwind-merge'
	import { DollarSign, Pencil, RefreshCw, SquareFunction } from 'lucide-svelte'

	interface Props {
		inputTransform: Record<string, any> | undefined
		id: string
		onEditInput?: (moduleId: string, key: string) => void
	}

	let { inputTransform, id, onEditInput }: Props = $props()

	const { flowStore, flowStateStore, testSteps, previewArgs } =
		getContext<FlowEditorContext | undefined>('FlowEditorContext') || {}

	onMount(() => {
		testSteps?.updateStepArgs(id, $flowStateStore, flowStore?.val, previewArgs?.val)
	})

	const input = $derived(testSteps?.getStepArgs(id)?.value)
</script>

<div class="p-4 pr-6 h-full overflow-y-auto">
	<ObjectViewer json={input} {inputTransform} {metaData} {editKey} />
</div>

{#snippet metaData(key: string)}
	{#if inputTransform?.[key]}
		<span
			class={twMerge(
				'inline-flex items-center h-4 border  px-1 rounded-[0.275rem] rounded-l-none border-l-0 gap-0.5',
				'text-2xs'
			)}
			title={inputTransform[key].type === 'javascript' ? inputTransform[key].expr : 'Static'}
		>
			{#if inputTransform[key].type === 'javascript'}
				<SquareFunction size={14} class="text-blue-500 -my-1 dark:text-blue-400" />
			{:else if inputTransform[key].type === 'static'}
				<DollarSign size={12} class="text-tertiary font-mono -my-1" />
			{/if}
			{#if testSteps?.isArgManuallySet(id, key)}
				<button
					onclick={() => {
						testSteps?.evalArg(id, key, $flowStateStore, flowStore?.val, previewArgs?.val)
					}}
					title="Re-evaluate input"
					class="-my-1 ml-0.5 hover:text-primary dark:hover:text-primary dark:text-gray-500 text-gray-300"
				>
					<RefreshCw size={12} />
				</button>
			{/if}
		</span>
	{/if}
{/snippet}

{#snippet editKey(key: string)}
	<button
		onclick={() => onEditInput?.(id, key)}
		class="h-4 w-fit items-center text-gray-300 dark:text-gray-500 hover:text-primary dark:hover:text-primary px-1 rounded-[0.275rem] align-baseline"
	>
		<Pencil size={12} class="-my-1 inline-flex items-center" />
	</button>
{/snippet}
