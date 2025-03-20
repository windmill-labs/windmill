<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowInputEditor from '$lib/components/flows/content/FlowInputEditor.svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import SideBarTab from '$lib/components/meltComponents/SideBarTab.svelte'
	import { History, Save } from 'lucide-svelte'
	import CaptureIcon from '$lib/components/triggers/CaptureIcon.svelte'
	import CaptureButton from './triggers/CaptureButton.svelte'
	import SavedInputsPicker from './SavedInputsPicker.svelte'
	import { createEventDispatcher } from 'svelte'
	import CaptureTable from './triggers/CaptureTable.svelte'
	import RefreshButton from './common/button/RefreshButton.svelte'

	export let runnableId: string = ''
	export let stablePathForCaptures: string = ''
	export let runnableType: any
	export let previewArgs: any
	export let isValid: boolean = true
	export let jsonView: boolean = false

	const dispatch = createEventDispatcher()

	export function refreshHistory() {
		historicInputs?.refresh()
	}

	export function resetSelected() {
		historicInputs?.resetSelected(true)
		savedInputsPicker?.resetSelected(true)
		captureTable?.resetSelected(true)
	}

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

	let savedInputsPicker: SavedInputsPicker | undefined = undefined
	let loading = false
	let captureTable: CaptureTable | undefined = undefined
	let historicInputs: HistoricInputs | undefined = undefined
	$: selectedTab, (dropdownItems = getDropdownItems())

	let inputPanelSize = 70
</script>

<div class="h-fit">
	<div class="relative z-[100000] w-full">
		<div class="absolute" style="right: calc({100 - inputPanelSize}%); top: -1px;">
			<SideBarTab expandRight={rightPanelOpen} {dropdownItems} fullMenu={true} noTrigger={true} />
		</div>
	</div>
	<Splitpanes class={!rightPanelOpen ? 'splitter-hidden' : ''}>
		<Pane bind:size={inputPanelSize} class="!overflow-visible" minSize={30}>
			<div class="relative w-full h-fit pr-12 pb-4" bind:clientHeight={rightHeight}>
				<slot />
			</div>
		</Pane>

		<Pane minSize={rightPanelOpen ? 30 : 0}>
			{#if rightPanelOpen}
				<div style="height: {rightHeight}px" class="border-t border-r pb-2">
					{#if selectedTab === 'history'}
						<FlowInputEditor title="History">
							<svelete:fragment slot="action">
								<div class="center-center">
									<RefreshButton {loading} on:click={() => historicInputs?.refresh()} />
								</div>
							</svelete:fragment>
							<HistoricInputs
								bind:loading
								bind:this={historicInputs}
								{runnableId}
								{runnableType}
								on:select={(e) => {
									dispatch('select', { payload: e.detail?.args, type: 'history' })
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
								bind:this={savedInputsPicker}
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
							<div class="h-full">
								<CaptureTable
									path={stablePathForCaptures}
									on:select={(e) => {
										dispatch('select', { payload: e.detail, type: 'captures' })
									}}
									bind:this={captureTable}
									isFlow={true}
									headless={true}
									addButton={false}
								/>
							</div>
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
