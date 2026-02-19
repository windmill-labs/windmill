<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowInputEditor from '$lib/components/flows/content/FlowInputEditor.svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import SideBarTab from '$lib/components/meltComponents/SideBarTab.svelte'
	import { History, Save } from 'lucide-svelte'
	import CaptureIcon from '$lib/components/triggers/CaptureIcon.svelte'
	import CaptureButton from './triggers/CaptureButton.svelte'
	import SavedInputsPicker from './SavedInputsPicker.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import CaptureTable from './triggers/CaptureTable.svelte'
	import RefreshButton from './common/button/RefreshButton.svelte'

	interface Props {
		runnableId?: string
		stablePathForCaptures?: string
		runnableType: any
		previewArgs: any
		isValid?: boolean
		jsonView?: boolean
		children?: import('svelte').Snippet
	}

	let {
		runnableId = '',
		stablePathForCaptures = '',
		runnableType,
		previewArgs,
		isValid = true,
		jsonView = false,
		children
	}: Props = $props()

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

	let rightHeight = $state(0)
	let selectedTab: 'history' | 'saved_inputs' | 'captures' | undefined = $state(undefined)
	let dropdownItems: any = $state()
	let rightPanelOpen = $state(false)

	let savedInputsPicker: SavedInputsPicker | undefined = $state(undefined)
	let captureTable: CaptureTable | undefined = $state(undefined)
	let historicInputs: HistoricInputs | undefined = $state(undefined)
	$effect(() => {
		selectedTab
		untrack(() => {
			dropdownItems = getDropdownItems()
		})
	})

	let inputPanelSize = $state(70)
</script>

<div class="h-fit">
	<div class="relative z-[100000] w-full">
		<div class="absolute" style="right: calc({100 - inputPanelSize}%); top: -1px;">
			<SideBarTab {dropdownItems} fullMenu={true} noTrigger={true} />
		</div>
	</div>
	<Splitpanes class={!rightPanelOpen ? 'splitter-hidden' : ''}>
		<Pane bind:size={inputPanelSize} class="!overflow-visible" minSize={30}>
			<div class="relative w-full h-fit pr-12 pb-4" bind:clientHeight={rightHeight}>
				{@render children?.()}
			</div>
		</Pane>

		<Pane minSize={rightPanelOpen ? 30 : 0}>
			{#if rightPanelOpen}
				<div style="height: {rightHeight}px" class="border-t border-r pb-2">
					{#if selectedTab === 'history'}
						<FlowInputEditor title="History">
							{#snippet action()}
								<svelete:fragment>
									<div class="center-center">
										<RefreshButton
											loading={historicInputs?.loading() ?? false}
											onClick={() => historicInputs?.refresh()}
										/>
									</div>
								</svelete:fragment>
							{/snippet}
							<HistoricInputs
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
							{#snippet action()}
								<svelete:fragment>
									<div class="center-center">
										<CaptureButton on:openTriggers small={true} />
									</div>
								</svelete:fragment>
							{/snippet}
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
