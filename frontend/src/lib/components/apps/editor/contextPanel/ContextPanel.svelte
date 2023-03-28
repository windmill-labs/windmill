<script lang="ts">
	import { classNames } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { getContext } from 'svelte'

	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { connectInput } from '../appUtils'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutput from './ComponentOutput.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import BackgroundScriptsOutput from './components/BackgroundScriptsOutput.svelte'
	import OutputHeader from './components/OutputHeader.svelte'

	const { connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

	const { search } = getContext<ContextPanelContext>('ContextPanel')
</script>

<PanelSection noPadding titlePadding="px-4 pt-2 pb-0.5" title="Outputs">
	<svelte:fragment slot="action">
		{#if $connectingInput.opened}
			<button
				class=""
				on:click|stopPropagation|preventDefault={() => ($connectingInput.opened = false)}
			>
				Cancel
			</button>
		{/if}
	</svelte:fragment>

	<div
		class={classNames(
			'bg-white w-full h-full z-30',
			$connectingInput.opened ? 'border-blue-500 border-t-2 border-r-2 bg-blue-50/50 z-50' : ''
		)}
	>
		<div class="min-w-[150px]">
			<div class="sticky z-10 top-0 left-0 w-full bg-white p-2 ">
				<div class="relative">
					<input
						bind:value={$search}
						class="px-2 pb-1 border border-gray-300 rounded-sm {search ? 'pr-8' : ''}"
						placeholder="Search outputs..."
					/>
					{#if search}
						<button
							class="absolute right-2 top-1/2 transform -translate-y-1/2 hover:bg-gray-200 rounded-full p-0.5"
							on:click|stopPropagation|preventDefault={() => ($search = '')}
						>
							<X size="14" />
						</button>
					{/if}
				</div>
			</div>

			<div class="flex flex-col gap-4">
				<div>
					<span class="text-xs font-bold p-2">State & Context</span>

					<OutputHeader selectable={false} id={'ctx'} name={'App Context'} first color="blue">
						<ComponentOutputViewer
							componentId={'ctx'}
							on:select={({ detail }) => {
								$connectingInput = connectInput($connectingInput, 'ctx', detail)
							}}
						/>
					</OutputHeader>

					<OutputHeader selectable={false} id={'state'} name={'State'} color="blue">
						<ComponentOutputViewer
							componentId={'state'}
							on:select={({ detail }) => {
								$connectingInput = connectInput($connectingInput, 'state', detail)
							}}
						/>
					</OutputHeader>
				</div>

				<div>
					<span class="text-sm font-bold p-2">Components</span>
					{#each $app.grid as gridItem, index (gridItem.id)}
						<ComponentOutput {gridItem} first={index === 0} />
					{/each}
				</div>
				<div>
					<span class="text-sm font-bold p-2">Background scripts</span>
					<div class="border-t">
						<BackgroundScriptsOutput />
					</div>
				</div>
			</div>
		</div>
	</div>
</PanelSection>
