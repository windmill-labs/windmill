<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowInputEditor from '$lib/components/flows/content/FlowInputEditor.svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import SideBarTab from '$lib/components/meltComponents/SideBarTab.svelte'
	import { ButtonType } from '$lib/components/common/button/model'
	import { twMerge } from 'tailwind-merge'
	import { ChevronRight, History, Save, Library } from 'lucide-svelte'
	import CaptureIcon from '$lib/components/triggers/CaptureIcon.svelte'
	import CaptureButton from './triggers/CaptureButton.svelte'
	import CapturesInputs from './CapturesInputs.svelte'
	import SavedInputsPicker from './SavedInputsPicker.svelte'

	export let runnableId
	export let runnableType: any
	export let flowPath
	export let previewArgs: any

	const getDropdownItems = () => {
		return [
			{
				label: 'History',
				onClick: () => {
					selectedTab = 'history'
				},
				icon: History,
				selected: selectedTab === 'history'
			},
			{
				label: 'Saved inputs',
				onClick: () => {
					selectedTab = 'saved_inputs'
				},
				icon: Save,
				selected: selectedTab === 'saved_inputs'
			},
			{
				label: 'Trigger captures',
				onClick: () => {
					selectedTab = 'captures'
				},
				icon: CaptureIcon,
				selected: selectedTab === 'captures'
			}
		]
	}

	let rightPanelSize = 0
	let rightHeight = 0
	let selectedTab: 'history' | 'saved_inputs' | 'captures' | undefined = undefined
	let dropdownItems: any

	$: selectedTab, (dropdownItems = getDropdownItems())

	function openRightPanel() {
		rightPanelSize = 40
		selectedTab = 'history'
	}

	function closeRightPanel() {
		rightPanelSize = 0
		selectedTab = undefined
	}

	function toggleRightPanel() {
		if (rightPanelSize > 0) {
			closeRightPanel()
		} else {
			openRightPanel()
		}
	}
</script>

<div class="h-fit">
	<Splitpanes>
		<Pane class="relative">
			<div class="absolute -right-0 z-50 bg-surface">
				<SideBarTab {dropdownItems} fullMenu={!!selectedTab}>
					<svelte:fragment slot="close button">
						<button
							on:click={() => {
								toggleRightPanel()
							}}
							title={selectedTab ? 'Close' : 'Open'}
							class={twMerge(
								ButtonType.ColorVariants.blue.contained,
								!!selectedTab ? 'rounded-tl-md border-l border-t' : 'rounded-md border'
							)}
						>
							<div class="p-2 center-center">
								{#if selectedTab}
									<ChevronRight size={14} />
								{:else}
									<div class="flex flex-row gap-2">
										<Library size={14} />
										<p class="text-2xs">Inputs library</p>
									</div>
								{/if}
							</div>
						</button>
					</svelte:fragment>
				</SideBarTab>
			</div>
			<div class="min-h-[40vh] h-fit {selectedTab ? 'pr-8' : ''}" bind:clientHeight={rightHeight}>
				<slot />
			</div>
		</Pane>
		{#if rightPanelSize > 0}
			<Pane bind:size={rightPanelSize} minSize={30}>
				<div style="height: {rightHeight}px" class="border-t border-r">
					{#if selectedTab === 'history'}
						<FlowInputEditor title="History">
							<HistoricInputs {runnableId} {runnableType} on:select />
						</FlowInputEditor>
					{:else if selectedTab === 'saved_inputs'}
						<FlowInputEditor title="Saved inputs">
							<SavedInputsPicker {runnableId} {runnableType} on:select {previewArgs} />
						</FlowInputEditor>
					{:else if selectedTab === 'captures'}
						<FlowInputEditor title="Trigger captures">
							<svelete:fragment slot="action">
								<div class="center-center">
									<CaptureButton on:openTriggers small={true} />
								</div>
							</svelete:fragment>
							<CapturesInputs on:select {flowPath} />
						</FlowInputEditor>
					{/if}
				</div>
			</Pane>
		{/if}
	</Splitpanes>
</div>

<style>
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
