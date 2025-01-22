<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, type RunnableType } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { Edit, Trash2, Save } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Cell } from './table/index'
	import { clickOutside } from '$lib/utils'
	import SaveInputsButton from '$lib/components/SaveInputsButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import InfiniteList from './InfiniteList.svelte'
	import { twMerge } from 'tailwind-merge'

	export let flowPath: string | null = null
	export let previewArgs: any = undefined

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
		isNew?: boolean
		isDeleting?: boolean
	}

	let infiniteList: InfiniteList | null = null
	let draft = true
	let selectedInput: string | null = null
	let isEditing: EditableInput | null = null

	const dispatch = createEventDispatcher()

	$: runnableId = flowPath || undefined
	let runnableType: RunnableType | undefined = undefined
	$: runnableType = flowPath ? 'FlowPath' : undefined

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
			return await InputService.listInputs({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage
			})
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
			selectedInput = null
			selectedArgs = undefined
			dispatch('select', undefined)
		} else {
			selectedInput = input.id
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
		const elements = document.querySelectorAll('[data-schema-picker], [data-schema-picker] *')
		return elements ? (Array.from(elements) as HTMLElement[]) : []
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && isEditing) {
			setEditing(null)
			event.stopPropagation()
			event.preventDefault()
		} else if (event.key === 'Escape' && selectedInput) {
			selectedInput = null
			selectedArgs = undefined
			dispatch('select', undefined)
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

	$: $workspaceStore && flowPath && (infiniteList && initLoadInputs(), (draft = false))
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
					infiniteList?.loadData('refresh')
				}}
				showTooltip={true}
			/>
		</Popover>
	</div>
	<div class="grow min-h-0">
		{#if !draft}
			<InfiniteList
				bind:this={infiniteList}
				selectedItemId={selectedInput}
				on:error={(e) => handleError(e.detail)}
				on:select={(e) => handleSelect(e.detail)}
			>
				<svelte:fragment slot="columns">
					<colgroup>
						<col class="w-8" />
						<col />
					</colgroup>
				</svelte:fragment>
				<svelte:fragment let:item>
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
								{#if isEditing && isEditing.id === item.id}
									<form
										on:submit={() => {
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
								{#if item.created_by == $userStore?.username || $userStore?.is_admin || $userStore?.is_super_admin}
									<div class="items-center flex gap-2">
										{#if !isEditing || isEditing?.id !== item.id}
											<div class="group-hover:block hidden -my-2">
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
											btnClasses={'group-hover:block hidden -my-2'}
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
												isEditing?.id === item.id ? 'block' : 'group-hover:block hidden -my-2',
												'hover:text-white hover:bg-red-500 text-red-500'
											)}
											on:click={() => {
												infiniteList?.deleteItem(item.id)
											}}
										>
											<Trash2 class="w-4 h-4" />
										</Button>
									</div>
								{:else}
									<span class="text-xs text-tertiary">By {item.created_by}</span>
								{/if}
							</div>
						</div>
					</Cell>
				</svelte:fragment>
				<svelte:fragment slot="empty">
					<div class="text-center text-xs text-tertiary">No saved Inputs</div>
				</svelte:fragment>
			</InfiniteList>
		{/if}
	</div>
</div>
