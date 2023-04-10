<script lang="ts">
	import { Button } from '$lib/components/common'
	import { FlowService, ScriptService, type Input } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { displayDate } from '$lib/utils.js'
	import { createEventDispatcher } from 'svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { faSave, faTrash } from '@fortawesome/free-solid-svg-icons'

	export let hash: string | undefined = ''
	export let script_path: string | undefined = ''
	export let flow_path: string | undefined = ''

	// Are the current Inputs valid and able to be saved?
	export let isValid: boolean
	export let args: object

	let previousInputs: Input[] = []
	let savedInputs: Input[] = []

	let selectedInput: Input | null

	async function loadInputHistory() {
		if (hash) {
			previousInputs = await ScriptService.getScriptInputHistoryByHash({
				workspace: $workspaceStore!,
				hash,
				perPage: 10
			})
		} else if (script_path) {
			previousInputs = await ScriptService.getScriptInputHistoryByPath({
				workspace: $workspaceStore!,
				path: script_path,
				perPage: 10
			})
		} else if (flow_path) {
			previousInputs = await FlowService.getFlowInputHistoryByPath({
				workspace: $workspaceStore!,
				path: flow_path,
				perPage: 10
			})
		}
	}

	$: {
		if ($workspaceStore && (hash || script_path || flow_path)) {
			loadInputHistory()
		}
	}

	const dispatch = createEventDispatcher()

	const selectArgs = (selected_args: object) => {
		dispatch('selected_args', selected_args)
	}

	let savingInputs = false

	async function saveInput(args: object) {
		savingInputs = true

		savedInputs.push({
			name: 'Saved ' + displayDate(new Date()),
			args,
			created_by: 'You',
			started_at: 'Just now'
		})
		savedInputs = [...savedInputs]

		// if (hash) {
		// 	await ScriptService.saveScriptInputsByHash({
		// 		workspace: $workspaceStore!,
		// 		hash,
		// 		args
		// 	})
		// } else if (script_path) {
		// 	await ScriptService.saveScriptInputsByPath({
		// 		workspace: $workspaceStore!,
		// 		path: script_path,
		// 		args
		// 	})
		// } else if (flow_path) {
		// 	await FlowService.saveFlowInputsByPath({
		// 		workspace: $workspaceStore!,
		// 		path: flow_path,
		// 		args
		// 	})
		// }

		savingInputs = false
	}

	async function deleteInput(input: Input) {
		savedInputs = savedInputs.filter((i) => i !== input)
		if (selectedInput === input) {
			selectedInput = null
		}
	}
</script>

<SplitPanesWrapper>
	<Splitpanes horizontal={true}>
		<Pane>
			<div class="w-full flex flex-col gap-4 p-4">
				<div class="w-full flex justify-between items-center">
					<span class="text-sm font-extrabold">Saved Inputs</span>
					<Button
						on:click={() => saveInput(args)}
						disabled={!isValid}
						loading={savingInputs}
						startIcon={{ icon: faSave }}
						color="blue"
						size="xs"
					>
						<span>Save Current Input</span>
					</Button>
				</div>

				<div class="w-full flex flex-col gap-2 p-2 h-full overflow-y-auto">
					{#if savedInputs.length > 0}
						{#each savedInputs as i}
							<Button
								color={selectedInput === i ? 'gray' : 'light'}
								btnClasses="w-full group h-12"
								on:click={() => (selectedInput = i)}
							>
								<div class="w-full h-full items-center justify-between flex gap-4 min-w-0">
									<small
										class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left"
									>
										{i.name}
									</small>

									<Button
										startIcon={{ icon: faTrash }}
										iconOnly={true}
										color="red"
										size="xs"
										btnClasses="group-hover:block hidden"
										on:click={() => deleteInput(i)}
									/>
								</div>
							</Button>
						{/each}
					{:else}
						<div class="text-center text-gray-500">No saved Inputs</div>
					{/if}
				</div>
			</div>
		</Pane>

		<Pane>
			<div class="w-full flex flex-col gap-4 p-4">
				<span class="text-sm font-extrabold">Previous Inputs</span>

				<div class="w-full flex flex-col gap-2 p-2 h-full overflow-y-auto">
					{#if previousInputs.length > 0}
						{#each previousInputs as i}
							<Button
								color={selectedInput === i ? 'gray' : 'light'}
								btnClasses="w-full h-12"
								on:click={() => (selectedInput = i)}
							>
								<div class="w-full h-full items-center flex gap-4 min-w-0">
									<small>{displayDate(i.started_at)}</small>
									<small
										class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left"
									>
										{i.created_by}
									</small>
								</div>
							</Button>
						{/each}
					{:else}
						<div class="text-center text-gray-500">No previous Inputs</div>
					{/if}
				</div>
			</div>
		</Pane>

		<Pane class="flex flex-col justify-between">
			<div class="w-full flex flex-col gap-4 p-4">
				<span class="text-sm font-extrabold">Preview</span>

				<div class="w-full h-full overflow-auto">
					{#if Object.keys(selectedInput?.args || {}).length > 0}
						<ObjectViewer json={selectedInput?.args} />
					{:else}
						<div class="text-center text-gray-500">
							Select an Input to preview scripts arguments
						</div>
					{/if}
				</div>
			</div>

			<div class="w-full flex flex-col p-4">
				<Button
					color="blue"
					btnClasses="w-full"
					on:click={() => selectArgs(selectedInput?.args)}
					disabled={Object.keys(selectedInput?.args || {}).length === 0}
				>
					Use Input
				</Button>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
