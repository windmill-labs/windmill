<script lang="ts">
	import { type RunnableType, type RunStartedBy } from '$lib/gen/index.js'
	import { sendUserToast } from '$lib/utils.js'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import HistoricList from './HistoricList.svelte'
	import { TOP_LEVEL_CATEGORIES } from './runHistory'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		runnableId?: string | undefined
		runnableType?: RunnableType | undefined
		selected?: string | undefined
		showAuthor?: boolean
		placement?: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end'
		limitPayloadSize?: boolean
		searchArgs?: Record<string, any> | undefined
		/** Workspace to read run history from; defaults to the nav workspace. */
		workspace?: string
		/** Kinds of runs to list; top-level runs by default. */
		startedBy?: RunStartedBy[]
	}

	let {
		runnableId = undefined,
		runnableType = undefined,
		selected = $bindable(undefined),
		showAuthor = false,
		placement = 'top-end',
		limitPayloadSize = false,
		searchArgs = undefined,
		workspace = undefined,
		startedBy = TOP_LEVEL_CATEGORIES
	}: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)

	let historicList: HistoricList | undefined = $state(undefined)
	const dispatch = createEventDispatcher()

	async function handleSelected(data: any) {
		if (selected === data.id) {
			resetSelected(true)
			return
		}
		selected = data.id
		if (data.payloadData === 'WINDMILL_TOO_BIG') {
			const fullPayload = await data.getFullPayload?.()
			dispatch('select', { args: fullPayload, jobId: data.id })
		} else {
			dispatch('select', {
				args: structuredClone($state.snapshot(data.payloadData)),
				jobId: data.id
			})
		}
	}

	export function loading(): boolean {
		return historicList?.loading() ?? false
	}

	onDestroy(() => {
		resetSelected(false)
	})

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selected) {
			resetSelected(true)
			event.stopPropagation()
			event.preventDefault()
		}
	}

	function handleError(e: { type: string; error: any }) {
		if (e.type === 'load') {
			sendUserToast(`Failed to load input history: ${e.error}`, true)
		}
	}

	export function refresh(clearCurrentRuns: boolean = false) {
		historicList?.refresh(clearCurrentRuns)
	}

	export function resetSelected(dispatchEvent?: boolean) {
		selected = undefined
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-full max-h-full min-h-0 w-full flex flex-col gap-2 relative">
	<div class="min-h-0 grow" data-schema-picker>
		<HistoricList
			bind:this={historicList}
			on:error={(e) => handleError(e.detail)}
			on:select={(e) => handleSelected(e.detail)}
			{runnableId}
			{runnableType}
			{selected}
			{showAuthor}
			{placement}
			{limitPayloadSize}
			{searchArgs}
			{startedBy}
			workspace={ws}
		/>
	</div>
</div>
