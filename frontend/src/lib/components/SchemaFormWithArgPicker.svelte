<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowInputEditor from '$lib/components/flows/content/FlowInputEditor.svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import SideBarTab from '$lib/components/meltComponents/SideBarTab.svelte'
	import { History, Save } from 'lucide-svelte'
	import CaptureIcon from '$lib/components/triggers/CaptureIcon.svelte'
	import CaptureButton from './triggers/CaptureButton.svelte'
	import CapturesInputs from './CapturesInputs.svelte'
	import SavedInputsPicker from './SavedInputsPicker.svelte'
	import { createEventDispatcher } from 'svelte'
	import { fly } from 'svelte/transition'

	export let runnableId: string = ''
	export let runnableType: any
	export let flowPath: string = ''
	export let previewArgs: any
	export let isValid: boolean = true
	export let jsonView: boolean = false

	const dispatch = createEventDispatcher()

	const getDropdownItems = () => {
		return [
			{
				label: 'History',
				onClick: () => {
					if (selectedTab === 'history') {
						selectedTab = undefined
						rightPanelOpen = false
					} else {
						selectedTab = 'history'
						rightPanelOpen = true
					}
				},
				icon: History,
				selected: selectedTab === 'history'
			},
			{
				label: 'Saved inputs',
				onClick: () => {
					if (selectedTab === 'saved_inputs') {
						selectedTab = undefined
						rightPanelOpen = false
					} else {
						selectedTab = 'saved_inputs'
						rightPanelOpen = true
					}
				},
				icon: Save,
				selected: selectedTab === 'saved_inputs'
			},
			{
				label: 'Trigger captures',
				onClick: () => {
					if (selectedTab === 'captures') {
						selectedTab = undefined
						rightPanelOpen = false
					} else {
						selectedTab = 'captures'
						rightPanelOpen = true
					}
				},
				icon: CaptureIcon,
				selected: selectedTab === 'captures'
			}
		]
	}

	let rightHeight = 0
	let selectedTab: 'history' | 'saved_inputs' | 'captures' | undefined = undefined
	let dropdownItems: any
	let rightPanelOpen = false

	$: selectedTab, (dropdownItems = getDropdownItems())
</script>

<div class="h-fit">
	<Splitpanes class={!rightPanelOpen ? 'splitter-hidden' : ''}>
		<Pane class="!overflow-visible" size={70} minSize={30}>
			<div class="relative w-full h-fit pr-12 pb-4" bind:clientHeight={rightHeight}>
				<div class="absolute -right-[1px] -top-[1px] z-50">
					<SideBarTab {dropdownItems} fullMenu={true} noTrigger={true} />
				</div>
				<slot />
			</div>
		</Pane>

		<Pane minSize={rightPanelOpen ? 30 : 0}>
			{#if rightPanelOpen}
				<div
					transition:fly={{ duration: 100, x: -200 }}
					style="height: {rightHeight}px"
					class="border-t border-r pb-2"
				>
					{#if selectedTab === 'history'}
						<FlowInputEditor title="History">
							<HistoricInputs
								{runnableId}
								{runnableType}
								on:select={(e) => {
									dispatch('select', { payload: e.detail, type: 'history' })
								}}
							/>
						</FlowInputEditor>
					{:else if selectedTab === 'saved_inputs'}
						<FlowInputEditor title="Saved inputs">
							<SavedInputsPicker
								{isValid}
								{runnableId}
								{runnableType}
								{previewArgs}
								on:select={(e) => {
									dispatch('select', { payload: e.detail, type: 'saved' })
								}}
								{jsonView}
							/>
						</FlowInputEditor>
					{:else if selectedTab === 'captures'}
						<FlowInputEditor title="Trigger captures">
							<svelete:fragment slot="action">
								<div class="center-center">
									<CaptureButton on:openTriggers small={true} />
								</div>
							</svelete:fragment>
							<CapturesInputs
								{flowPath}
								on:select={(e) => {
									dispatch('select', { payload: e.detail, type: 'captures' })
								}}
							/>
						</FlowInputEditor>
					{/if}
				</div>
			{/if}
		</Pane>
	</Splitpanes>
</div>

<style>
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
