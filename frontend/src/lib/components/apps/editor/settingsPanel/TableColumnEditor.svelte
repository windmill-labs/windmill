<script lang="ts">
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import TableColumnWizard from '$lib/components/wizards/TableColumnWizard.svelte'
	import { Button } from '$lib/components/common'
	import { Settings } from 'lucide-svelte'
	import type { TableColumns } from '../component/components'
	import type { BadgeColor } from '$lib/components/common/badge/model'

	export let columns: Record<string, TableColumns>
	export let componentId: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let object: {
		result: Array<object>
	} = {
		result: []
	}

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			Object.entries(observableOutputs).forEach(([k, output]) => {
				object[k] = undefined
				output?.subscribe(
					{
						id: 'alloutputs' + componentId + '-' + k,
						next: (value) => {
							object[k] = value
						}
					},
					object[k]
				)
			})
		}
	}

	$: subscribeToAllOutputs($worldStore?.outputsById?.[componentId])

	function getAllKeys(arrayOfObjects: Record<string, any>[]) {
		const allKeysSet = arrayOfObjects.reduce((acc: Set<any>, obj) => {
			Object.keys(obj).forEach((key: string) => acc.add(key))
			return acc
		}, new Set())

		return Array.from(allKeysSet)
	}

	$: keys = getAllKeys(object?.result ?? []) as string[]

	$: {
		// Extract the current keys in columns for easy checking
		const existingKeysInColumns = columns ? new Set(Object.keys(columns)) : new Set()

		if (columns === undefined) {
			columns = {}
		}

		// Add new keys to columns
		keys.forEach((key) => {
			if (!existingKeysInColumns.has(key)) {
				columns[key] = {
					showColumn: {
						fieldType: 'boolean',
						type: 'static',
						value: true
					},
					type: {
						type: 'oneOf',
						selected: 'text',
						labels: {
							text: 'Text',
							badge: 'Badge'
						},
						tooltip: 'TODO',
						configuration: {
							text: {},
							badge: {
								color: {
									fieldType: 'select',
									type: 'static',

									selectOptions: [
										'gray',
										'red',
										'yellow',
										'green',
										'blue',
										'indigo',
										'purple',
										'pink'
									],
									value: 'gray' as BadgeColor
								}
							}
						}
					}
				}
			}
		})

		// Remove absent keys from columns
		for (const key in columns) {
			if (!keys.includes(key)) {
				delete columns[key]
			}
		}
	}
</script>

<PanelSection title="Columns">
	<div class=" border rounded-md text-xs flex flex-col w-full">
		{#each keys as key}
			<div
				class="hover:bg-blue-200 dark:hover:bg-blue-900 flex flex-row justify-between items-center p-2 transition-all cursor-pointer"
			>
				{key}
				{#if columns?.[key]}
					<TableColumnWizard bind:columns={columns[key]} id={componentId}>
						<svelte:fragment slot="trigger">
							<Button color="light" size="xs2" nonCaptureEvent={true}>
								<div class="flex flex-row items-center gap-2 text-xs font-normal">
									<Settings size={16} />
								</div>
							</Button>
						</svelte:fragment>
					</TableColumnWizard>
				{/if}
			</div>
		{/each}
	</div>
</PanelSection>
