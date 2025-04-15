<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'
	import { isObject } from '$lib/utils'

	export let columns: string[] = []
	export let id: string | undefined

	let remainingColumns: string[] = []

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	let result = []

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			observableOutputs?.['result']?.subscribe(
				{
					id: 'quickadd-' + id + '-result',
					next: (value) => {
						result = value
					}
				},
				result
			)
		}
	}

	function updateRemainingColumns(result: any[], columns: string[]) {
		if (Array.isArray(result) && result?.length > 0) {
			const allKeysSet: Set<string> = result.reduce((acc, obj) => {
				if (isObject(obj)) {
					Object.keys(obj).forEach((key) => acc.add(key))
				}
				return acc
			}, new Set<string>())

			remainingColumns = Array.from(allKeysSet).filter((x: string) => !columns?.includes(x))
		}
	}

	$: id && subscribeToAllOutputs($worldStore?.outputsById?.[id])
	$: updateRemainingColumns(result, columns)
</script>

{#if remainingColumns.length > 0}
	<div class="text-xs font-semibold">Quick add </div>

	<div class="flex flex-row gap-2 items-center flex-wrap">
		{#each remainingColumns as column}
			<Button
				on:click={() => dispatch('add', column)}
				size="xs2"
				color="light"
				variant="border"
				startIcon={{ icon: Plus }}
			>
				{column}
			</Button>
		{/each}
	</div>
{/if}
