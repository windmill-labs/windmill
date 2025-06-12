<script lang="ts">
	import SavedInputsPicker from '$lib/components/SavedInputsPicker.svelte'
	import { createEventDispatcher, tick } from 'svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'
	import { type RunnableType } from '$lib/gen/types.gen.js'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Section from '$lib/components/Section.svelte'
	import SaveInputsButton from '$lib/components/SaveInputsButton.svelte'
	import { Button } from './common'
	import { ExternalLink, Search } from 'lucide-svelte'
	import { Popover } from './meltComponents'
	import SchemaForm from './SchemaForm.svelte'
	import MultiSelect from './multiselect/MultiSelectWrapper.svelte'
	const dispatch = createEventDispatcher()

	interface Props {
		scriptHash?: string | null
		scriptPath?: string | null
		flowPath?: string | null
		inputSelected?: 'saved' | 'history' | undefined
		jsonView?: boolean
		schema: any
		// Are the current Inputs valid and able to be saved?
		isValid: boolean
		args: object
	}

	let {
		scriptHash = null,
		scriptPath = null,
		flowPath = null,
		inputSelected = $bindable(undefined),
		jsonView = false,
		schema,
		isValid,
		args
	}: Props = $props()

	export function resetSelected() {
		historicInputs?.resetSelected(true)
		savedInputsPicker?.resetSelected(true)
	}

	let savedArgs: any = undefined
	let savedInputsPicker: SavedInputsPicker | undefined = $state(undefined)
	let historicInputs: HistoricInputs | undefined = $state(undefined)

	let runnableId = $derived(scriptHash || scriptPath || flowPath || undefined)

	let runnableType: RunnableType | undefined = $derived(
		scriptHash ? 'ScriptHash' : scriptPath ? 'ScriptPath' : flowPath ? 'FlowPath' : undefined
	)

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

	let searchArgs = $state({})
	let appliedSearchArgs = $state({})

	let searchArgsFields = $state([] as string[])

	let emptySearchArgs = $derived(Object.keys(appliedSearchArgs).length === 0)

	let filteredSchema = $derived({
		properties: Object.fromEntries(
			Object.entries(schema?.properties ?? {}).filter(([key]) => searchArgsFields.includes(key))
		)
	})
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
				{#snippet action()}
					<SaveInputsButton
						{args}
						disabled={!isValid || jsonView}
						{runnableId}
						{runnableType}
						on:update={() => {
							savedInputsPicker?.refresh()
						}}
					/>
				{/snippet}

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
				{#snippet action()}
					<div class="flex space-x-2">
						<Popover
							class="w-fit"
							usePointerDownOutside
							closeOnOtherPopoverOpen
							on:click={(e) => {
								e.stopPropagation()
							}}
							floatingConfig={{ placement: 'top-end' }}
						>
							{#snippet trigger()}
								<Button
									variant="contained"
									size="xs2"
									color={emptySearchArgs ? 'light' : 'dark'}
									nonCaptureEvent
									title="Search history"
									iconOnly={emptySearchArgs}
									endIcon={{ icon: Search }}
								>
									{emptySearchArgs ? '' : 'Active filters'}
								</Button>
							{/snippet}

							{#snippet content()}
								<div id="multi-select-search"></div>
								<div class="p-2 overflow-auto max-h-[400px] min-h-[300px] w-[400px]">
									<div class="flex items-center flex-wrap gap-x-2 justify-between">
										<div class="text-sm text-secondary">Search by args</div>
										<div class="flex flex-wrap gap-x-2">
											{#if !emptySearchArgs}
												<Button
													on:click={async () => {
														searchArgs = {}
														appliedSearchArgs = {}
														await tick()
														historicInputs?.refresh(true)
													}}
													variant="contained"
													size="xs2"
													color="light">Reset filters</Button
												>
											{/if}
											<Button
												on:click={async () => {
													appliedSearchArgs = structuredClone(searchArgs)
													await tick()
													historicInputs?.refresh(true)
												}}
												endIcon={{ icon: Search }}
												variant="contained"
												size="xs2"
												color="dark">Search</Button
											>
										</div>
									</div>
									<div class="my-2">
										<MultiSelect
											topPlacement
											target="#multi-select-search"
											placeholder="arg fields to filter on"
											items={Object.keys(schema?.properties ?? {})}
											bind:value={searchArgsFields}
										/>
									</div>
									{#key filteredSchema}
										<SchemaForm shouldHideNoInputs schema={filteredSchema} bind:args={searchArgs} />
									{/key}
									{#if searchArgsFields.length === 0}
										<div class="text-secondary text-sm my-8 mx-auto">No filters</div>
									{/if}
								</div>
							{/snippet}
						</Popover>
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
					</div>
				{/snippet}
				<HistoricInputs
					bind:this={historicInputs}
					{runnableId}
					{runnableType}
					on:select={(e) => {
						if (e.detail) savedInputsPicker?.resetSelected()
						selectArgs(e.detail?.args, e.detail ? 'history' : undefined)
					}}
					searchArgs={emptySearchArgs ? undefined : appliedSearchArgs}
					showAuthor
					placement="top-end"
				/>
			</Section>
		</Pane>
	</Splitpanes>
</div>
