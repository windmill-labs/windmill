<script lang="ts">
	import { InputService, type RunnableType } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy } from 'svelte'
	import InfiniteList from './InfiniteList.svelte'
	import JobSchemaPicker from './schema/JobSchemaPicker.svelte'

	export let runnableId: string | undefined
	export let runnableType: RunnableType | undefined
	export let selected: string | undefined = undefined

	let infiniteList: InfiniteList | undefined = undefined
	let loadInputsPageFn:
		| ((page: number, perPage: number, discovery: boolean) => Promise<any>)
		| undefined = undefined

	export function refresh() {
		if (infiniteList) {
			infiniteList.loadData('refresh')
		}
	}
	let cachedArgs: Record<string, any> = {}
	let items: any[] = []
	let potentialItems: any[] = []
	let perPageBind = 0
	let lastChecked: string | undefined = undefined

	let interval: NodeJS.Timeout | undefined = undefined
	function initLoadInputs() {
		interval && clearInterval(interval)
		interval = setInterval(() => {
			refresh()
		}, 10000)
		loadInputsPageFn = async (page: number, perPage: number, discovery: boolean) => {
			const request = InputService.getInputHistory({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage,
				includePreview: true,
				// If it is discovery, then we would like to fetch all values
				since: !discovery ? lastChecked : undefined
			})
			if (!discovery) lastChecked = new Date().toJSON()
			const inputs = await request

			const inputsWithPayload = await Promise.all(
				inputs.map(async (input) => {
					if (cachedArgs[input.id]) {
						return {
							...input,
							payloadData: cachedArgs[input.id]
						}
					}
					const payloadData = await loadArgsFromHistory(input.id, undefined, false)
					if (payloadData === 'WINDMILL_TOO_BIG') {
						return {
							...input,
							payloadData: 'WINDMILL_TOO_BIG',
							getFullPayload: () => loadArgsFromHistory(input.id, undefined, true)
						}
					}
					cachedArgs[input.id] = payloadData
					return {
						...input,
						payloadData
					}
				})
			)
			if (!discovery) {
				// Add new items to beginning
				items.unshift(...inputsWithPayload)
				// We need to know when to apply potential items,
				// it only happens when InfiniteList decides to expand list
				//
				// We cannot apply potential items right after fetch,
				// because that would trigger expansion in list
				// and old items will be loading by 10 every reload
				if (perPageBind != perPage) items.push(...potentialItems), (perPageBind = perPage)
				return items
			} else {
				// Save discovered items to buffer and apply later
				potentialItems = inputsWithPayload
				return potentialItems
			}
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	onDestroy(() => {
		interval && clearInterval(interval)
	})

	async function loadArgsFromHistory(
		id: string | undefined,
		input: boolean | undefined,
		allowLarge: boolean
	): Promise<any> {
		if (!id) return
		const payloadData = await InputService.getArgsFromHistoryOrSavedInput({
			jobOrInputId: id,
			workspace: $workspaceStore!,
			input,
			allowLarge
		})
		return payloadData
	}

	$: $workspaceStore && runnableId && runnableType && infiniteList && initLoadInputs()
</script>

<InfiniteList bind:this={infiniteList} selectedItemId={selected} on:error on:select>
	<svelte:fragment slot="columns">
		<colgroup>
			<col class="w-8" />
			<col class="w-16" />
			<col />
		</colgroup>
	</svelte:fragment>
	<svelte:fragment let:item let:hover>
		<JobSchemaPicker
			job={item}
			selected={selected === item.id}
			hovering={hover}
			payloadData={item.payloadData}
		/>
	</svelte:fragment>
	<svelte:fragment slot="empty">
		<div class="text-center text-tertiary text-xs py-2">
			{runnableId ? 'No previous inputs' : 'Save draft to see previous runs'}
		</div>
	</svelte:fragment>
</InfiniteList>
