<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'

	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { connectInput } from '../appUtils'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutput from './ComponentOutput.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import BackgroundScriptsOutput from './components/BackgroundScriptsOutput.svelte'
	import OutputHeader from './components/OutputHeader.svelte'
	import { ClearableInput } from '../../../common'

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
				<ClearableInput bind:value={$search} placeholder="Search outputs..." />
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
