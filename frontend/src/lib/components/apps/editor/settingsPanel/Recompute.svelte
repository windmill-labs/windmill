<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	export let recomputeIds: string[] | undefined = undefined
	export let ownId: string
	export let title: string = 'Trigger runnables on success'
	export let tooltip: string =
		'Select components to recompute after this runnable has successfully run'
	export let documentationLink: string =
		'https://www.windmill.dev/docs/apps/app-runnable-panel#recompute-others'

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	function onChange(checked: boolean, id: string) {
		if (checked) {
			recomputeIds = [...(recomputeIds ?? []), id]
		} else {
			recomputeIds = recomputeIds?.filter((x) => x !== id)
		}
	}
</script>

<PanelSection {title} {tooltip} {documentationLink}>
	{#if Object.keys($runnableComponents ?? {}).filter((id) => id !== ownId).length > 0}
		<table class="divide-y border w-full">
			<thead class="bg-surface-secondary">
				<tr>
					<th scope="col" class="px-2 py-2 text-left text-xs font-medium text-tertiary">
						Component
					</th>
					<th scope="col" class="px-4 py-2 text-left text-xs font-medium text-tertiary">
						Recompute
					</th>
				</tr>
			</thead>
			<tbody>
				{#each Object.keys($runnableComponents ?? {}).filter((id) => id !== ownId) as id}
					<tr>
						<td class="whitespace-nowrap px-2 text-xs">
							<Badge color="indigo">{id}</Badge>
						</td>
						<td class="relative whitespace-nowrap px-4">
							<Toggle
								class="windmillapp my-1"
								size="xs"
								on:change={(e) => onChange(e.detail, id)}
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
