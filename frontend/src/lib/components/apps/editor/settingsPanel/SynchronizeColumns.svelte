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

	function haveSameStringElements(arr1: string[], arr2: string[]): boolean {
		if (arr1.length !== arr2.length) {
			return false
		}

		const sortedArr1 = [...arr1].sort()
		const sortedArr2 = [...arr2].sort()

		for (let i = 0; i < sortedArr1.length; i++) {
			if (sortedArr1[i] !== sortedArr2[i]) {
				return false
			}
		}

		return true
	}

	$: shouldDisplaySyncButton = !haveSameStringElements(columns, remainingColumns)
</script>

{#if shouldDisplaySyncButton}
	<div class="flex flex-row gap-2 items-center flex-wrap">
		<Button
			on:click={() => {
				dispatch('add', remainingColumns)
			}}
			size="xs2"
			color="dark"
			startIcon={{ icon: Columns }}
		>
			Synchronize columns
		</Button>
	</div>
{/if}
