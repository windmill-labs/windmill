<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, RunnableType, type CreateInput } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'
	import { classNames, displayDate, sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { ArrowLeftIcon, Edit, ExternalLink, Save, X } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import TimeAgo from './TimeAgo.svelte'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null
	export let canSaveInputs: boolean = true

	let runnableId: string | undefined = scriptPath || flowPath || undefined
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
			args
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
				created_by: '',
				created_at: new Date().toISOString(),
				is_public: false,
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
					name: input.name,
					is_public: input.is_public
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

	const selectArgs = (selected_args: any) => {
		dispatch('selected_args', selected_args)
	}
</script>

<div class="min-w-[300px] h-full">
	<Splitpanes horizontal={true}>
		<Pane>
			<div class="w-full flex flex-col gap-4 p-2">
				<div class="w-full flex justify-between items-center gap-4 flex-wrap">
					<span class="text-sm font-extrabold flex-shrink-0"
						>Saved Inputs <Tooltip
							>Shared inputs are available to anyone with access to the script</Tooltip
						></span
					>
					{#if canSaveInputs}
						<Button
							on:click={() => saveInput(args)}
							disabled={!isValid}
							loading={savingInputs}
							startIcon={{ icon: Save }}
							color="light"
							size="xs"
						>
							<span>Save Current Input</span>
						</Button>
					{/if}
				</div>

				<div class="w-full flex flex-col gap-2 h-full overflow-y-auto p">
					{#if savedInputs.length > 0}
						{#each savedInputs as i}
							<button
								class={classNames(
									`w-full flex items-center group justify-between gap-4 py-2 px-4 text-left border rounded-md hover:bg-surface-hover transition-all`,
									selectedInput === i ? 'border-blue-500 bg-blue-50 dark:bg-blue-900' : ''
								)}
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
											<input type="text" bind:value={i.name} class="text-secondary" />
										</form>
									{:else}
										<small
											class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left"
										>
											{i.name}
										</small>
									{/if}
									{#if i.created_by == $userStore?.username || $userStore?.is_admin || $userStore?.is_super_admin}
										<div class="items-center flex gap-2">
											{#if !i.isEditing}
												<div class="group-hover:block hidden -my-2">
													<Toggle
														size="xs"
														options={{ right: 'shared' }}
														bind:checked={i.is_public}
														on:change={() => {
															updateInput(i)
														}}
													/>
												</div>
											{/if}

											<Button
												loading={i.isSaving}
												color="light"
												size="xs"
												variant="border"
												spacingSize="xs2"
												btnClasses={'group-hover:block hidden -my-2'}
												on:click={(e) => {
													e.stopPropagation()
													i.isEditing = !i.isEditing
													if (!i.isEditing) {
														updateInput(i)
														i.isSaving = false
													}
												}}
											>
												<Edit class="w-4 h-4" />
											</Button>
											<Button
												color="red"
												size="xs"
												spacingSize="xs2"
												variant="border"
												btnClasses={i.isEditing ? 'block' : 'group-hover:block hidden -my-2'}
												on:click={() => deleteInput(i)}
											>
												<X class="w-4 h-4" />
											</Button>
										</div>
									{:else}
										<span class="text-xs text-tertiary">By {i.created_by}</span>
									{/if}
								</div>
							</button>
						{/each}
					{:else}
						<div class="text-center text-tertiary">No saved Inputs</div>
					{/if}
				</div>
			</div>
		</Pane>

		<Pane>
			<div class="w-full flex flex-col gap-4 p-2">
				<span class="text-sm font-extrabold">Previous runs</span>

				<div class="w-full flex flex-col gap-1 p-0 h-full overflow-y-auto">
					{#if previousInputs.length > 0}
						{#each previousInputs as i}
							<button
								class={classNames(
									`w-full flex items-center justify-between gap-4 py-2 px-4 text-left border rounded-sm hover:bg-surface-hover transition-a`,
									selectedInput === i ? 'border-blue-500 bg-blue-50 dark:bg-blue-900' : ''
								)}
								on:click={() => {
									if (selectedInput === i) {
										selectedInput = null
									} else {
										selectedInput = i
									}
								}}
							>
								<div
									class="w-full h-full items-center text-xs font-normal grid grid-cols-8 gap-4 min-w-0"
								>
									<div class="">
										<div class="rounded-full w-2 h-2 {i.success ? 'bg-green-400' : 'bg-red-400'}" />
									</div>
									<div class="col-span-2">
										{i.created_by}
									</div>
									<div
										class="whitespace-nowrap col-span-3 font-normal overflow-hidden text-ellipsis flex-shrink text-center"
									>
										<TimeAgo date={i.created_at ?? ''} />
									</div>
									<div class="col-span-2">
										<a
											target="_blank"
											href="/run/{i.id}?workspace={$workspaceStore}"
											class="text-right float-right text-secondary"
											title="See run detail in a new tab"
										>
											<ExternalLink size={16} />
										</a>
									</div>
								</div>
							</button>
						{/each}
					{:else}
						<div class="text-center text-tertiary">No previous Runs</div>
					{/if}
				</div>
			</div>
		</Pane>

		<Pane>
			<div class="h-full overflow-hidden min-h-0 flex flex-col justify-between">
				<div class="w-full flex flex-col min-h-0 gap-2 px-2 py-2 grow">
					<div class="text-sm font-extrabold">Preview</div>
					<div class="w-full flex flex-col">
						<Button
							color="blue"
							btnClasses="w-full"
							size="sm"
							spacingSize="xl"
							on:click={() => selectArgs(selectedInput?.args)}
							disabled={Object.keys(selectedInput?.args || {}).length === 0}
						>
							<ArrowLeftIcon class="w-4 h-4 mr-2" />
							Use Input
						</Button>
					</div>
					<div class="w-full min-h-0 grow overflow-auto">
						{#if Object.keys(selectedInput?.args || {}).length > 0}
							<div class="border overflow-auto h-full p-2">
								<ObjectViewer json={selectedInput?.args} />
							</div>
						{:else}
							<div class="text-center text-tertiary">
								Select an Input to preview scripts arguments
							</div>
						{/if}
					</div>
				</div>
			</div></Pane
		>
	</Splitpanes>
</div>
