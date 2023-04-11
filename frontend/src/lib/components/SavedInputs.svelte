<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, RunnableType, type CreateInput } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { displayDate, sendUserToast } from '$lib/utils.js'
	import { faEdit, faSave, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null

	let runnableId: string | undefined = scriptHash || scriptPath || flowPath || undefined
	let runnableType: RunnableType | undefined = scriptHash
		? RunnableType.SCRIPT_HASH
		: scriptPath
		? RunnableType.SCRIPT_PATH
		: flowPath
		? RunnableType.FLOW_PATH
		: undefined

	// Are the current Inputs valid and able to be saved?
	export let isValid: boolean
	export let args: object

	let previousInputs: Input[] = []
	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
	}
	let savedInputs: EditableInput[] = []

	let selectedInput: Input | null

	async function loadInputHistory() {
		previousInputs = await InputService.getInputHistory({
			workspace: $workspaceStore!,
			runnableId,
			runnableType,
			perPage: 10
		})
	}

	async function loadSavedInputs() {
		savedInputs = await InputService.listInputs({
			workspace: $workspaceStore!,
			runnableId,
			runnableType,
			perPage: 10
		})
	}

	let savingInputs = false

	async function saveInput(args: object) {
		savingInputs = true

		const requestBody: CreateInput = {
			name: 'Saved ' + displayDate(new Date()),
			args,
			created_by: 'You'
		}

		try {
			let id = await InputService.createInput({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				requestBody
			})

			const input = {
				id,
				...requestBody
			}
			savedInputs = [input, ...savedInputs]
		} catch (err) {
			console.error(err)
			sendUserToast(`Failed to save Input: ${err}`, true)
		}

		savingInputs = false
	}

	async function updateInput(input: EditableInput) {
		input.isSaving = true

		try {
			await InputService.updateInput({
				workspace: $workspaceStore!,
				requestBody: {
					id: input.id,
					name: input.name
				}
			})
		} catch (err) {
			console.error(err)
			sendUserToast(`Failed to update Input: ${err}`, true)
		}

		input.isSaving = false
	}

	async function deleteInput(input: Input) {
		try {
			await InputService.deleteInput({
				workspace: $workspaceStore!,
				input: input.id
			})
			savedInputs = savedInputs.filter((i) => i.id !== input.id)
			if (selectedInput === input) {
				selectedInput = null
			}
		} catch (err) {
			console.error(err)
			sendUserToast(`Failed to delete Input: ${err}`, true)
		}
	}

	$: {
		if ($workspaceStore && (scriptHash || scriptPath || flowPath)) {
			loadInputHistory()
			loadSavedInputs()
		}
	}

	const dispatch = createEventDispatcher()

	const selectArgs = (selected_args: object) => {
		dispatch('selected_args', selected_args)
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
								on:click={() => {
									if (!i.isEditing) {
										if (selectedInput === i) {
											selectedInput = null
										} else {
											selectedInput = i
										}
									}
								}}
							>
								<div class="w-full h-full items-center justify-between flex gap-1 min-w-0">
									{#if i.isEditing}
										<form
											on:submit={() => {
												updateInput(i)
												i.isEditing = false
												i.isSaving = false
											}}
											class="w-full"
										>
											<input type="text" bind:value={i.name} class="text-gray-700" />
										</form>
									{:else}
										<small
											class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left"
										>
											{i.name}
										</small>
									{/if}

									<div class="flex gap-1">
										<Button
											startIcon={{ icon: i.isEditing ? faSave : faEdit }}
											loading={i.isSaving}
											iconOnly={true}
											color="gray"
											size="xs"
											btnClasses={i.isEditing ? 'block' : 'group-hover:block hidden'}
											on:click={(e) => {
												e.stopPropagation()
												i.isEditing = !i.isEditing
												if (!i.isEditing) {
													updateInput(i)
													i.isSaving = false
												}
											}}
										/>

										<Button
											startIcon={{ icon: faTrash }}
											iconOnly={true}
											color="red"
											size="xs"
											btnClasses={i.isEditing ? 'block' : 'group-hover:block hidden'}
											on:click={() => deleteInput(i)}
										/>
									</div>
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
								on:click={() => {
									if (selectedInput === i) {
										selectedInput = null
									} else {
										selectedInput = i
									}
								}}
							>
								<div class="w-full h-full items-center flex gap-4 min-w-0">
									<small
										class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left"
									>
										{i.name}
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
