<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, type RunnableType, type CreateInput } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'

	import { classNames, displayDate, sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import { ArrowLeftIcon, Edit, Save, X } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null
	export let canSaveInputs: boolean = true

	// Are the current Inputs valid and able to be saved?
	export let isValid: boolean
	export let args: object

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
	}

	let previousInputs: Input[] | undefined = undefined
	let savedInputs: EditableInput[] | undefined = undefined
	let selectedInput: Input | null
	let savingInputs = false
	const dispatch = createEventDispatcher()

	$: runnableId = scriptHash || scriptPath || flowPath || undefined

	let runnableType: RunnableType | undefined = undefined
	$: runnableType = scriptHash
		? 'ScriptHash'
		: scriptPath
		? 'ScriptPath'
		: flowPath
		? 'FlowPath'
		: undefined

	let hasAlreadyFailed = false
	async function loadInputHistory() {
		try {
			previousInputs = await InputService.getInputHistory({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				perPage: 10
			})
		} catch (e) {
			console.error(e)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			sendUserToast(`Failed to load input history: ${e}`, true)
		}
	}

	async function loadSavedInputs() {
		savedInputs = await InputService.listInputs({
			workspace: $workspaceStore!,
			runnableId,
			runnableType,
			perPage: 10
		})
	}

	async function saveInput(args: object) {
		savingInputs = true

		const requestBody: CreateInput = {
			name: 'Saved ' + displayDate(new Date()),
			args: args as any
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
			savedInputs = [input, ...(savedInputs ?? [])]
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
			savedInputs = (savedInputs ?? []).filter((i) => i.id !== input.id)
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

	let previewArgs: any = undefined

	function selectArgs(selected_args: any) {
		previewArgs = selected_args
	}

	async function loadLargeArgs(
		id: string | undefined,
		input: boolean | undefined,
		allowLarge: boolean
	): Promise<any> {
		if (!id) return
		return await InputService.getArgsFromHistoryOrSavedInput({
			jobOrInputId: id,
			workspace: $workspaceStore!,
			input,
			allowLarge
		})
	}
</script>

<div class="min-w-[300px] h-full">
	<Splitpanes horizontal={true}>
		<Pane>
			<div class="w-full flex flex-col gap-4 p-2">
				<div class="w-full flex justify-between items-center gap-4 flex-wrap">
					<span class="text-sm font-semibold flex-shrink-0"
						>Saved Inputs test<Tooltip
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

				<div class="w-full flex flex-col gap-1 h-full overflow-y-auto p">
					{#if savedInputs === undefined}
						<Skeleton layout={[[8]]} />
					{:else if savedInputs?.length > 0}
						{#each savedInputs as i}
							<button
								class={classNames(
									`w-full flex items-center text-sm group justify-between gap-4 py-1.5 px-4 text-left border rounded-sm hover:bg-surface-hover transition-all`,
									selectedInput === i ? 'border-blue-500 bg-blue-50 dark:bg-blue-900' : ''
								)}
								on:click={async () => {
									if (!i.isEditing) {
										if (selectedInput === i) {
											selectedInput = null
										} else {
											selectedInput = i
										}
									}
									selectArgs(await loadLargeArgs(i.id, true, false))
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
			<div class="h-full overflow-hidden min-h-0 flex flex-col justify-between">
				<div class="w-full flex flex-col min-h-0 gap-2 px-2 py-2 grow">
					<div class="w-full flex flex-col">
						<Button
							color="blue"
							btnClasses="w-full"
							size="sm"
							spacingSize="xl"
							on:click={async () => {
								dispatch('selected_args', await loadLargeArgs(selectedInput?.id, undefined, false))
							}}
							disabled={!selectedInput}
						>
							<ArrowLeftIcon class="w-4 h-4 mr-2" />
							Use Input
						</Button>
					</div>
					<div class="w-full min-h-0 grow overflow-auto">
						{#if typeof previewArgs == 'string' && previewArgs == 'WINDMILL_TOO_BIG'}
							<div class="text-secondary mt-2">
								Payload too big to preview but can still be loaded</div
							>
						{:else if Object.keys(previewArgs || {}).length > 0}
							<div class=" overflow-auto h-full p-2">
								<ObjectViewer json={previewArgs} />
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
