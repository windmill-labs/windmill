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

	let hasState: boolean = false
</script>

<PanelSection noPadding titlePadding="px-2 pt-2" title="Outputs">
	<div
		class={classNames(
			'bg-white w-full h-full z-30',
			$connectingInput.opened ? 'border-blue-500 border-t-2 border-r-2 bg-blue-50/50 z-50' : ''
		)}
	>
		<div class="min-w-[150px]">
			<div class="sticky z-10 top-0 left-0 w-full bg-white p-1.5">
				<div class="relative w-full">
					<input
						bind:value={$search}
						class="!border-gray-200 !rounded-md !text-xs !pr-6"
						placeholder="Search outputs..."
					/>
					{#if $search !== ''}
						<button
							class="absolute right-2 top-1/2 transform -translate-y-1/2 hover:bg-gray-500 bg-gray-300 transition-all rounded-full p-0.5"
							on:click|stopPropagation|preventDefault={() => ($search = '')}
						>
							<X size="10" color="white" />
						</button>
					{/if}
				</div>
			</div>

			<div class="flex flex-col gap-4">
				<div>
					<span class="text-xs font-semibold text-gray-800 p-2">State & Context</span>

					<OutputHeader selectable={false} id={'ctx'} name={'App Context'} first color="blue">
						<ComponentOutputViewer
							componentId={'ctx'}
							on:select={({ detail }) => {
								$connectingInput = connectInput($connectingInput, 'ctx', detail)
							}}
						/>
					</OutputHeader>

					<OutputHeader
						selectable={false}
						id={'state'}
						name={'State'}
						color="blue"
						disabled={!hasState}
					>
						<ComponentOutputViewer
							bind:hasContent={hasState}
							componentId={'state'}
							on:select={({ detail }) => {
								$connectingInput = connectInput($connectingInput, 'state', detail)
							}}
						/>
					</OutputHeader>
				</div>

				<div>
					<span class="text-xs font-semibold text-gray-800 p-2">Components</span>
					{#each $app.grid as gridItem, index (gridItem.id)}
						<ComponentOutput {gridItem} first={index === 0} />
					{/each}
				</div>
				<div>
					<span class="text-xs font-semibold text-gray-800 p-2">Background scripts</span>
					<BackgroundScriptsOutput />
				</div>
			</div>
		</div>
	</div>
</PanelSection>
