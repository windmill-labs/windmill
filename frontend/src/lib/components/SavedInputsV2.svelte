<script lang="ts">
	import SavedInputsPicker from '$lib/components/SavedInputsPicker.svelte'
	import { createEventDispatcher } from 'svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import { type RunnableType } from '$lib/gen/types.gen.js'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Section from '$lib/components/Section.svelte'
	import SaveInputsButton from '$lib/components/SaveInputsButton.svelte'
	import { Button } from './common'
	import { ExternalLink } from 'lucide-svelte'
	const dispatch = createEventDispatcher()

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null
	export let inputSelected: 'saved' | 'history' | undefined = undefined
	export let jsonView: boolean = false

	// Are the current Inputs valid and able to be saved?
	export let isValid: boolean
	export let args: object

	export function resetSelected() {
		historicInputs?.resetSelected(true)
		savedInputsPicker?.resetSelected(true)
	}

	let savedArgs: any = undefined
	let runnableType: RunnableType | undefined = undefined
	let savedInputsPicker: SavedInputsPicker | undefined = undefined
	let historicInputs: HistoricInputs | undefined = undefined

	$: runnableId = scriptHash || scriptPath || flowPath || undefined
	$: runnableType = scriptHash
		? 'ScriptHash'
		: scriptPath
		? 'ScriptPath'
		: flowPath
		? 'FlowPath'
		: undefined

	function selectArgs(selected_args: any, type: 'saved' | 'history' | undefined) {
		if (selected_args) {
			if (!inputSelected) {
				savedArgs = args
			}
			inputSelected = type
			dispatch('selected_args', selected_args)
		} else if (savedArgs) {
			inputSelected = type
			dispatch('selected_args', savedArgs)
		}
	}
</script>

<div class="min-w-[300px] h-full flex flex-col">
	<Splitpanes horizontal={true}>
		<Pane class="px-4 py-2 h-full">
			<Section
				label="Saved Inputs"
				tooltip="Shared inputs are available to anyone with access to the script"
				wrapperClass="h-full"
				small={true}
			>
				<svelte:fragment slot="action">
					<SaveInputsButton
						{args}
						disabled={!isValid || jsonView}
						{runnableId}
						{runnableType}
						on:update={() => {
							savedInputsPicker?.refresh()
						}}
					/>
				</svelte:fragment>

				<SavedInputsPicker
					bind:this={savedInputsPicker}
					previewArgs={args}
					{runnableId}
					{runnableType}
					{isValid}
					on:select={(e) => {
						if (e.detail) historicInputs?.resetSelected()
						selectArgs(e.detail, e.detail ? 'saved' : undefined)
					}}
					noButton={true}
				/>
			</Section>
		</Pane>

		<Pane class="px-4 py-4 h-full">
			<Section label="History" wrapperClass="h-full" small={true}>
				<svelte:fragment slot="action">
					<Button
						size="xs2"
						color="light"
						btnClasses="!text-tertiary"
						endIcon={{ icon: ExternalLink }}
						on:click={() => {
							window.open(`/runs/${runnableId}`, '_blank')
						}}
					>
						All runs
					</Button>
				</svelte:fragment>
				<HistoricInputs
					bind:this={historicInputs}
					{runnableId}
					{runnableType}
					on:select={(e) => {
						if (e.detail) savedInputsPicker?.resetSelected()
						selectArgs(e.detail?.args, e.detail ? 'history' : undefined)
					}}
					showAuthor
					placement="top-start"
				/>
			</Section>
		</Pane>
	</Splitpanes>
</div>
