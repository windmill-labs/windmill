<script lang="ts">
	import { X } from 'lucide-svelte'
	import { getContext, setContext } from 'svelte'
	import { writable } from 'svelte/store'

	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { connectInput } from '../appUtils'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutput from './ComponentOutput.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import BackgroundScriptsOutput from './components/BackgroundScriptsOutput.svelte'
	import MinMaxButton from './components/MinMaxButton.svelte'
	import OutputHeader from './components/OutputHeader.svelte'

	const { connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

	let search = writable<string>('')
	let expanded = writable(false)

	setContext<ContextPanelContext>('ContextPanel', {
		search,
		manuallyOpened: writable<Record<string, boolean>>({}),
		hasResult: writable<Record<string, boolean>>({}),
		expanded
	})
</script>

<PanelSection noPadding titlePadding="px-4 pt-2 pb-0.5" title="Outputs">
	<div class="bg-white w-full h-full z-30">
		<div class="min-w-[150px]">
			<div class="sticky z-10 top-0 left-0 w-full bg-white p-2">
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

			<div class="p-1 ">
				<MinMaxButton bind:expanded={$expanded} />
			</div>

			<div class="flex flex-col gap-4">
				<div>
					<span class="text-xs font-bold p-2">State & Context</span>

					<OutputHeader id={'ctx'} name={'App Context'} first color="blue">
						<ComponentOutputViewer
							componentId={'ctx'}
							on:select={({ detail }) => {
								$connectingInput = connectInput($connectingInput, 'ctx', detail)
							}}
						/>
					</OutputHeader>

					<OutputHeader id={'state'} name={'State'} color="blue">
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
