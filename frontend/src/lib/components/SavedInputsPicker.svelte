<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, type RunnableType } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { Edit, X, Save } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DataTable from './table/DataTable.svelte'
	import { Row, Cell } from './table/index'
	import { twMerge } from 'tailwind-merge'
	import { clickOutside } from '$lib/utils'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
	}

	let savedInputs: EditableInput[] | undefined = undefined
	let selectedInput: EditableInput | null
	let hasMore = false
	let page = 1
	let perPage = 20

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
	async function loadSavedInputs(refresh = false) {
		if (refresh) {
			savedInputs = undefined
			hasMore = false
			page = 1
			return loadSavedInputs()
		}

		try {
			const newInputs = await InputService.listInputs({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage
			})

			savedInputs = [...(savedInputs ?? []), ...newInputs]
			hasMore = savedInputs.length === perPage * page
			page++
		} catch (e) {
			console.error(e)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			sendUserToast(`Failed to load saved inputs: ${e}`, true)
			hasMore = savedInputs ? savedInputs.length === perPage * page : false
			page++
		}
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
			loadSavedInputs(true)
		}
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

	let selectedArgs: any = undefined
	async function handleSelect(input: EditableInput) {
		if (input.isEditing) return

		if (selectedInput === input) {
			selectedInput = null
			selectedArgs = undefined
			dispatch('select', undefined)
		} else {
			selectedInput = input
			selectedArgs = await loadLargeArgs(input.id, true, false)
			dispatch('select', selectedArgs)
		}
	}

	onDestroy(() => {
		selectedInput = null
		selectedArgs = undefined
		dispatch('select', undefined)
	})
</script>

<div
	class="w-full flex flex-col gap-1 h-full overflow-y-auto p"
	use:clickOutside
	on:click_outside={() => {
		selectedInput = null
		selectedArgs = undefined
		dispatch('select', undefined)
	}}
>
	{#if savedInputs === undefined}
		<Skeleton layout={[[8]]} />
	{:else if savedInputs?.length > 0}
		<DataTable
			size="xs"
			infiniteScroll
			{hasMore}
			tableFixed={true}
			on:loadMore={() => {
				loadSavedInputs()
			}}
		>
			<colgroup>
				<col class="w-8" />
				<col />
			</colgroup>

			<tbody class="w-full overflow-y-auto">
				{#each savedInputs as i}
					<Row
						on:click={async () => {
							await handleSelect(i)
						}}
						class={twMerge(
							selectedInput === i ? 'bg-surface-selected' : 'hover:bg-surface-hover',
							'cursor-pointer group rounded-md'
						)}
					>
						<Cell>
							<div class="center-center">
								<Save size={12} />
							</div>
						</Cell>
						<Cell>
							<div
								class="w-full flex items-center text-sm justify-between gap-4 px-4 text-left transition-all"
							>
								<div class="w-full h-full items-center justify-between flex gap-1 min-w-0 p-1">
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
							</div>
						</Cell>
					</Row>
				{/each}
			</tbody>
		</DataTable>
	{:else}
		<div class="text-center text-xs text-tertiary">No saved Inputs</div>
	{/if}
</div>
