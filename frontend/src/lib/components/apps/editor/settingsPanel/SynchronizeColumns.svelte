<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Columns } from 'lucide-svelte'
	import { isObject } from '$lib/utils'

	export let columns: string[] = []

	let remainingColumns: string[] = []

	const { worldStore, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	let result = []

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			Object.entries(observableOutputs).forEach(([k, output]) => {
				output?.subscribe(
					{
						id: 'alloutputs-quickadd' + $selectedComponent?.[0] + '-' + k,
						next: (value) => {
							if (k === 'result') {
								result = value
							}
						}
					},
					result
				)
			})
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

			remainingColumns = Array.from(allKeysSet).filter((x: string) => !columns?.includes(x) ?? true)
		}
	}

	$: $selectedComponent?.[0] &&
		subscribeToAllOutputs($worldStore?.outputsById?.[$selectedComponent?.[0]])
	$: updateRemainingColumns(result, columns)
</script>

{#if remainingColumns.length > 0}
	<div class="flex flex-row gap-2 items-center flex-wrap">
		<Button
			on:click={() => {
				remainingColumns.forEach((column) => dispatch('add', column))
			}}
			size="xs2"
			color="dark"
			startIcon={{ icon: Columns }}
		>
			Synchronize columns
		</Button>
	</div>
{/if}
