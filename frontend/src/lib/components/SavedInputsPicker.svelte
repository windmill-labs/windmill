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
	import SaveInputsButton from '$lib/components/SaveInputsButton.svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let flowPath: string | null = null
	export let previewArgs: any = undefined

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
	}

	let savedInputs: EditableInput[] | undefined = undefined
	let selectedInput: EditableInput | null
	let hasMore = false
	let page = 1
	let perPage = 10

	const dispatch = createEventDispatcher()

	$: runnableId = flowPath || undefined
	let runnableType: RunnableType | undefined = undefined
	$: runnableType = flowPath ? 'FlowPath' : undefined

	let hasAlreadyFailed = false
	async function loadSavedInputs(refresh = false) {
		hasMore = hasMore

		try {
			const newInputs = await InputService.listInputs({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page: 1,
				perPage: (refresh || !hasMore ? page : page + 1) * perPage
			})
			if (
				refresh &&
				savedInputs &&
				savedInputs?.length > 0 &&
				newInputs.length === savedInputs?.length &&
				newInputs.every((i, index) => i.id === savedInputs?.[index]?.id)
			) {
				return
			}
			savedInputs = newInputs
			hasMore = savedInputs.length === perPage * page
			page = Math.floor(newInputs.length / perPage) + 1
		} catch (e) {
			console.error(e)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			sendUserToast(`Failed to load saved inputs: ${e}`, true)
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

	let draft = true
	$: {
		if ($workspaceStore && flowPath) {
			loadSavedInputs(true)
			draft = false
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

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-schema-picker], [data-schema-picker] *')
		) as HTMLElement[]
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selectedInput) {
			selectedInput = null
			selectedArgs = undefined
			dispatch('select', undefined)
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<div
	class="w-full flex flex-col gap-1 h-full overflow-y-auto p"
	use:clickOutside={{ capture: false, exclude: getPropPickerElements }}
	on:click_outside={() => {
		selectedInput = null
		selectedArgs = undefined
		dispatch('select', undefined)
	}}
>
	<div>
		<Popover class="w-full" placement="bottom" disablePopup={flowPath && previewArgs}>
			<svelte:fragment slot="text">
				{#if !flowPath}
					Save draft first before you can save inputs
				{:else if !previewArgs}
					Add inputs before saving
				{/if}
			</svelte:fragment>
			<SaveInputsButton
				{runnableId}
				{runnableType}
				args={previewArgs ?? {}}
				disabled={!previewArgs || !flowPath}
				on:update={() => {
					loadSavedInputs(true)
				}}
				showTooltip={true}
			/>
		</Popover>
	</div>
	<div class="grow min-h-0">
		{#if savedInputs === undefined && !draft}
			<Skeleton layout={[[8]]} />
		{:else if savedInputs && savedInputs.length > 0}
			<DataTable
				size="xs"
				infiniteScroll
				{hasMore}
				tableFixed={true}
				on:loadMore={() => {
					console.log('dbg load more')
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
</div>
