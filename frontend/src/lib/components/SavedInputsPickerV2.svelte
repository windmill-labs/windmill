<script lang="ts">
	import { Button } from '$lib/components/common'
	import { InputService, type Input, type RunnableType } from '$lib/gen/index.js'
	import { userStore, workspaceStore } from '$lib/stores.js'

	import { classNames, sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher } from 'svelte'
	import { Edit, X } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
	}

	let previousInputs: Input[] | undefined = undefined
	let savedInputs: EditableInput[] | undefined = undefined
	let selectedInput: EditableInput | null
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
</script>

<div class="w-full flex flex-col gap-1 h-full overflow-y-auto p">
	{#if savedInputs === undefined}
		<Skeleton layout={[[8]]} />
	{:else if savedInputs?.length > 0}
		{#each savedInputs as i}
			<button
				class={classNames(
					`w-full flex items-center text-sm group justify-between gap-4 py-1.5 px-4 text-left border rounded-sm transition-all`,
					selectedInput === i ? 'bg-surface-selected' : 'hover:bg-surface-hover'
				)}
				on:click={async () => {
					await handleSelect(i)
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
						<small class="whitespace-nowrap overflow-hidden text-ellipsis flex-shrink text-left">
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
