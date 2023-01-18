<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'

	export let recomputeIds: string[] | undefined = undefined
	export let ownId: string

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	function onChange(
		event: Event & {
			currentTarget: EventTarget & HTMLInputElement
		},
		id: string
	) {
		if (event.currentTarget.checked) {
			recomputeIds = [...(recomputeIds ?? []), id]
		} else {
			recomputeIds = recomputeIds?.filter((x) => x !== id)
		}
	}
</script>

<PanelSection title="Recompute others" tooltip="Select components to recompute after running this script">
	{#if Object.keys($runnableComponents ?? {}).filter((id) => id !== ownId).length > 0}
		<table class="divide-y divide-gray-300 border w-full">
			<thead class="bg-gray-50">
				<tr>
					<th scope="col" class="px-2 py-2 text-left text-xs font-medium text-gray-500">
						Component
					</th>
					<th scope="col" class="px-4 py-2 text-left text-xs font-medium text-gray-500">
						Recompute
					</th>
				</tr>
			</thead>
			<tbody>
				{#each Object.keys($runnableComponents ?? {}).filter((id) => id !== ownId) as id}
					<tr>
						<td class="whitespace-nowrap px-4 py-2 text-xs">
							<Badge color="dark-indigo">{id}</Badge>
						</td>
						<td class="relative whitespace-nowrap px-4 py-2 ">
							<input
								type="checkbox"
								on:change={(event) => onChange(event, id)}
								checked={recomputeIds?.includes(id)}
							/>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{:else}
		<div class="text-xs">No components to recompute. Create one and select it here.</div>
	{/if}
</PanelSection>
