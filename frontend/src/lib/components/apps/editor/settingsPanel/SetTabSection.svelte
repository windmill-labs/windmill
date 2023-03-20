<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { allItems } from '../../utils'
	import PanelSection from './common/PanelSection.svelte'

	export let setTab: { id: string; index: number }

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	const tabComponents = allItems($app.grid, $app.subgrids).filter(
		(component) => component.data.type === 'tabscomponent'
	)
</script>

<PanelSection title="Set tab on success">
	<div>
		<div class="flex flex-row gap-2 w-full">
			<div class="flex flex-col">
				<label for="tabId" class="text-xs font-semibold">Tab ID</label>

				<select
					id="tabId"
					bind:value={setTab.id}
					class="border border-gray-300 rounded-md p-1 !w-16"
				>
					{#each tabComponents as tabComponent}
						<option value={tabComponent.data.id}>
							{tabComponent.data.id}
						</option>
					{/each}
				</select>
			</div>
			<div class="flex flex-col">
				<label for="tabIndex" class="text-xs font-semibold">Tab Index</label>
				<select
					id="tabIndex"
					bind:value={setTab.index}
					class="border border-gray-300 rounded-md p-1 !w-16"
				>
					{#each tabComponents as tabComponent}
						{#if tabComponent.data.id === setTab.id}
							{#each Array(tabComponent.data.numberOfSubgrids) as _, i}
								<option value={i}>{i}</option>
							{/each}
						{/if}
					{/each}
				</select>
			</div>
		</div>
	</div>
</PanelSection>
