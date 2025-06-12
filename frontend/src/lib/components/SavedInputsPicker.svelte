<script lang="ts">
	import { run } from 'svelte/legacy'

	import { Button } from '$lib/components/common'
	import { InputService, type Input, type RunnableType } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { Edit, Trash2, Save } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Cell } from './table/index'
	import SaveInputsButton from '$lib/components/SaveInputsButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import InfiniteList from './InfiniteList.svelte'
	import { twMerge } from 'tailwind-merge'
	import SavedInputsPickerViewer from './SavedInputsPickerViewer.svelte'

	interface Props {
		previewArgs?: any
		runnableId?: string | undefined
		runnableType?: RunnableType | undefined
		isValid?: boolean
		noButton?: boolean
		jsonView?: boolean
		limitPayloadSize?: boolean
	}

	let {
		previewArgs = undefined,
		runnableId = undefined,
		runnableType = undefined,
		isValid = false,
		noButton = false,
		jsonView = false,
		limitPayloadSize = false
	}: Props = $props()

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
		isNew?: boolean
		isDeleting?: boolean
		payloadData?: any
		getFullPayload?: () => Promise<any>
	}

	let infiniteList: InfiniteList | null = $state(null)
	let draft = $state(true)
	let selectedInput: string | null = $state(null)
	let isEditing: EditableInput | null = $state(null)
	let viewerOpen = $state(false)
	let openStates: Record<string, boolean> = $state({})
	let clientWidth: number = $state(0)

	const dispatch = createEventDispatcher()

	async function updateInput(input: EditableInput | null) {
		if (!input) return
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

	function initLoadInputs() {
		const loadInputsPageFn = async (page: number, perPage: number) => {
			const inputs = await InputService.listInputs({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage
			})
			const inputsWithPayload = await Promise.all(
				inputs.map(async (input) => {
					const payloadData = await loadLargeArgs(input.id, undefined, false)
					if (payloadData === 'WINDMILL_TOO_BIG') {
						return {
							...input,
							payloadData: 'WINDMILL_TOO_BIG',
							getFullPayload: () => loadLargeArgs(input.id, undefined, true)
						}
					}
					return {
						...input,
						payloadData
					}
				})
			)
			return inputsWithPayload
		}
		infiniteList?.setLoader(loadInputsPageFn)

		const deleteInputFn = async (id: string) => {
			await InputService.deleteInput({
				workspace: $workspaceStore!,
				input: id
			})
		}
		infiniteList?.setDeleteItemFn(deleteInputFn)
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

		if (selectedInput === input.id) {
			resetSelected(true)
		} else {
			selectedInput = input.id
			if (input.payloadData === 'WINDMILL_TOO_BIG') {
				const fullPayload = await input.getFullPayload?.()
				dispatch('select', fullPayload)
			} else {
				selectedArgs = structuredClone(input.payloadData ?? {})
				dispatch('select', selectedArgs)
			}
		}
	}

	onDestroy(() => {
		resetSelected(true)
	})

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && isEditing) {
			setEditing(null)
			event.stopPropagation()
			event.preventDefault()
		} else if (event.key === 'Escape' && selectedInput) {
			resetSelected(true)
			event.stopPropagation()
			event.preventDefault()
		}

		if (event.key === 'Enter' && isEditing) {
			updateInput(isEditing).then(() => {
				setEditing(null)
			})
			event.stopPropagation()
			event.preventDefault()
		}
	}

	function setEditing(input: EditableInput | null) {
		isEditing = input
		dispatch('isEditing', !!input)
	}

	function handleError(error: { type: string; error: any }) {
		if (error.type === 'delete') {
			sendUserToast(`Failed to delete saved input: ${error.error}`, true)
		} else if (error.type === 'load') {
			sendUserToast(`Failed to load saved inputs: ${error.error}`, true)
		}
	}

	export function refresh() {
		infiniteList?.loadData('refresh')
	}

	export function resetSelected(dispatchEvent?: boolean) {
		selectedInput = null
		selectedArgs = undefined
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}

	function updateViewerOpenState(itemId: string, isOpen: boolean) {
		openStates[itemId] = isOpen
		viewerOpen = Object.values(openStates).some((state) => state)
	}

	run(() => {
		$workspaceStore &&
			runnableId &&
			runnableType &&
			(infiniteList && initLoadInputs(), (draft = false))
	})
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="w-full flex flex-col gap-1 h-full overflow-y-auto">
	{#if !noButton}
		<div>
			<Popover class="w-full" placement="bottom" disablePopup={runnableId && previewArgs}>
				{#snippet text()}
					{#if !runnableId}
						Save draft first before you can save inputs
					{:else if !previewArgs}
						Add inputs before saving
					{/if}
				{/snippet}
				<SaveInputsButton
					{runnableId}
					{runnableType}
					args={previewArgs ?? {}}
					disabled={!previewArgs || !runnableId || !isValid || jsonView}
					on:update={() => {
						refresh()
					}}
					showTooltip={true}
				/>
			</Popover>
		</div>
	{/if}
	<div class="grow min-h-0" data-schema-picker>
		{#if !draft}
			<InfiniteList
				bind:this={infiniteList}
				selectedItemId={selectedInput}
				on:error={(e) => handleError(e.detail)}
				on:select={(e) => handleSelect(e.detail)}
			>
				{#snippet columns()}
					<colgroup>
						<col class="w-8" />
						<col />
					</colgroup>
				{/snippet}
				{#snippet children({ item, hover })}
					{@const editOptions =
						item.created_by == $userStore?.username ||
						$userStore?.is_admin ||
						$userStore?.is_super_admin}
					<Cell>
						<div class="center-center">
							<Save size={12} />
						</div>
					</Cell>
					<Cell>
						<div
							class="w-full flex items-center text-sm justify-between gap-4 py-1 text-left transition-all"
							bind:clientWidth
						>
							<div class="w-full h-full items-center justify-between flex gap-1 min-w-0">
								{#if isEditing && isEditing.id === item.id}
									<form
										onsubmit={() => {
											updateInput(isEditing)
											setEditing(null)
										}}
										class="w-full"
									>
										<input type="text" bind:value={isEditing.name} class="text-secondary" />
									</form>
								{:else}
									<small
										class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left"
									>
										{item.name}
									</small>
								{/if}

								<div class="items-center flex gap-2">
									<SavedInputsPickerViewer
										payloadData={item.payloadData}
										{limitPayloadSize}
										{hover}
										{viewerOpen}
										on:openChange={(e) => {
											updateViewerOpenState(item.id, e.detail)
										}}
										maxWidth={clientWidth}
										{editOptions}
									/>
									{#if editOptions}
										{#if !isEditing || isEditing?.id !== item.id}
											<div class={hover || openStates[item.id] ? 'block -my-2' : 'hidden'}>
												<Toggle
													size="xs"
													options={{ right: 'shared' }}
													checked={item.is_public}
													on:change={(e) => {
														updateInput({ ...item, is_public: e.detail })
													}}
												/>
											</div>
										{/if}

										<Button
											loading={isEditing?.id === item.id && isEditing?.isSaving}
											color="light"
											size="xs"
											variant="border"
											spacingSize="xs2"
											btnClasses={hover || openStates[item.id] ? 'block -my-2 ' : 'hidden'}
											on:click={(e) => {
												e.stopPropagation()
												if (isEditing?.id === item.id) {
													updateInput(isEditing)
													setEditing(null)
												} else {
													setEditing(item)
												}
											}}
										>
											<Edit class="w-4 h-4" />
										</Button>
										<Button
											color="light"
											size="xs"
											spacingSize="xs2"
											variant="contained"
											btnClasses={twMerge(
												isEditing?.id === item.id || hover || openStates[item.id]
													? 'block -my-2'
													: 'hidden',
												'bg-transparent hover:text-white hover:bg-red-500 text-red-500'
											)}
											on:click={() => {
												infiniteList?.deleteItem(item.id)
											}}
										>
											<Trash2 class="w-4 h-4" />
										</Button>
									{:else}
										<span class="text-xs text-tertiary px-2 w-28 truncate" title={item.created_by}
											>{item.created_by}</span
										>
									{/if}
								</div>
							</div>
						</div>
					</Cell>
				{/snippet}
				{#snippet empty()}
					<div class="text-center text-xs text-tertiary">No saved Inputs</div>
				{/snippet}
			</InfiniteList>
		{/if}
	</div>
</div>
