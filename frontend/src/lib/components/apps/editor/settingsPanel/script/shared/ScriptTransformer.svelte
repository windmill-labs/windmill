<script lang="ts">
	import type { AppEditorContext, InlineScript } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Plus, X } from 'lucide-svelte'

	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	export let appInput: { transformer?: InlineScript & { language: 'frontend' } }
	export let id: string

	$: checked = Boolean(appInput.transformer)
</script>

<div class="p-2">
	<div class="text-sm font-semibold justify-between flex flex-row items-center">
		<div class="flex flex-row items-center gap-2">
			Transformer

			<Tooltip wrapperClass="flex">
				{"A transformer is an optional frontend script that is executed right after the component's script whose purpose is to do lightweight transformation in the browser. It takes the previous computation's result as `result`"}
			</Tooltip>
		</div>
		<Button
			size="xs"
			color={checked ? 'red' : 'light'}
			variant="border"
			on:click={() => {
				if (appInput.transformer) {
					appInput.transformer = undefined
				} else {
					appInput.transformer = {
						language: 'frontend',
						content: 'return result'
					}
					$selectedComponentInEditor = id + '_transformer'
				}
			}}
		>
			<div class="flex flex-row gap-1 items-center">
				{#if checked}
					<X size={16} />
					Remove
				{:else}
					<Plus size={16} />
					Add
				{/if}
			</div>
		</Button>
	</div>
</div>
