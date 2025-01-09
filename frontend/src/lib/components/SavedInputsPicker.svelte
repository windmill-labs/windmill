<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, type RunnableType } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { Edit, X, Save } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { Cell } from './table/index'
	import { clickOutside } from '$lib/utils'
	import SaveInputsButton from '$lib/components/SaveInputsButton.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import InfiniteList from './InfiniteList.svelte'

	export let flowPath: string | null = null
	export let previewArgs: any = undefined

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
		isNew?: boolean
		isDeleting?: boolean
	}

	let infiniteList: InfiniteList | null = null

	let selectedInput: string | null = null

	const dispatch = createEventDispatcher()

	$: runnableId = flowPath || undefined
	let runnableType: RunnableType | undefined = undefined
	$: runnableType = flowPath ? 'FlowPath' : undefined

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

	let draft = true
	let loadInputsPageFn: ((page: number, perPage: number) => Promise<any>) | undefined = undefined
	let deleteInputFn: ((id: string) => Promise<any>) | undefined = undefined
	function initLoadInputs() {
		loadInputsPageFn = async (page: number, perPage: number) => {
			return await InputService.listInputs({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage
			})
		}

		deleteInputFn = async (id: string) => {
			await InputService.deleteInput({
				workspace: $workspaceStore!,
				input: id
			})
		}
		draft = false
	}
	$: $workspaceStore && flowPath && initLoadInputs()

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
		console.log('dbg handleSelect', input)
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
					console.log('dbg update', infiniteList?.loadData)
					infiniteList?.loadData(true)
				}}
				showTooltip={true}
			/>
		</Popover>
	</div>
	<div class="grow min-h-0">
		{#if !draft}
			<InfiniteList
				bind:this={infiniteList}
				loadInputs={loadInputsPageFn}
				deleteItemFn={deleteInputFn}
				selectedItemId={selectedInput}
				on:error={(e) => sendUserToast(`Failed to load saved inputs: ${e.detail}`, true)}
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
								{#if item.isEditing}
									<form
										on:submit={() => {
											updateInput(item)
											item.isEditing = false
											item.isSaving = false
										}}
										class="w-full"
									>
										<input type="text" value={item.name} class="text-secondary" />
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
										{#if !item.isEditing}
											<div class="group-hover:block hidden -my-2">
												<Toggle
													size="xs"
													options={{ right: 'shared' }}
													checked={item.is_public}
													on:change={() => {
														updateInput(item)
													}}
												/>
											</div>
										{/if}

										<Button
											loading={item.isSaving}
											color="light"
											size="xs"
											variant="border"
											spacingSize="xs2"
											btnClasses={'group-hover:block hidden -my-2'}
											on:click={(e) => {
												e.stopPropagation()
												item.isEditing = !item.isEditing
												if (!item.isEditing) {
													updateInput(item)
													item.isSaving = false
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
											btnClasses={item.isEditing ? 'block' : 'group-hover:block hidden -my-2'}
											on:click={() => {
												infiniteList?.deleteItem(item.id)
											}}
										>
											<X class="w-4 h-4" />
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
