<script lang="ts">
	import { isObject } from '$lib/utils'
	import SchemaPickerRow from '$lib/components/schema/SchemaPickerRow.svelte'
	import type { Capture } from '$lib/gen'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Webhook, Route, Unplug, Mail } from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'

	const dispatch = createEventDispatcher()

	export let testKind: 'main' | 'preprocessor' = 'main'
	export let isFlow = false
	export let flowPath: string | null = null
	export let scriptHash: string | null = null

	let selected: number | undefined = undefined

	let captures: Capture[] = []

	let page = 1
	let perPage = 10
	let hasMore = false

	async function refreshCaptures(path: string | null) {
		if (!path?.length) return
		captures = await CaptureService.listCaptures({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path: path ?? '',
			page,
			perPage
		})
		hasMore = captures.length === perPage
	}

	const captureKindToIcon: Record<string, any> = {
		webhook: Webhook,
		route: Route,
		email: Mail,
		websocket: Unplug,
		kafka: KafkaIcon
	}

	$: path = isFlow ? flowPath : scriptHash
	$: refreshCaptures(path)

	function handleSelect(capture: Capture) {
		if (selected === capture.id) {
			selected = undefined
			dispatch('select', undefined)
		} else {
			selected = capture.id
			dispatch('select', capture.payload)
		}
	}

	onDestroy(() => {
		selected = undefined
		dispatch('select', undefined)
	})

	function goToNextPage() {
		page++
		refreshCaptures(path)
	}

	function goToPreviousPage() {
		page--
		refreshCaptures(path)
	}
</script>

<div class="flex flex-col gap-1 grow overflow-y-auto">
	{#if captures.length === 0}
		<div class="text-xs text-secondary">No trigger tests yet</div>
	{:else}
		<DataTable
			size="xs"
			paginated
			on:next={goToNextPage}
			on:previous={goToPreviousPage}
			bind:currentPage={page}
			{hasMore}
			tableFixed={true}
		>
			<colgroup>
				<col class="w-8" />
				<col class="w-20" />
				<col />
			</colgroup>

			<tbody class="w-full overflow-y-auto">
				{#each captures as capture}
					{@const payload = isObject(capture.payload) ? capture.payload : {}}
					{@const triggerExtra = isObject(capture.trigger_extra) ? capture.trigger_extra : {}}
					{@const payloadData =
						testKind === 'preprocessor'
							? {
									...payload,
									...triggerExtra
							  }
							: payload}
					{@const captureIcon = captureKindToIcon[capture.trigger_kind]}
					<SchemaPickerRow
						date={capture.created_at}
						{payloadData}
						on:updateSchema
						on:applyArgs
						on:addPreprocessor
						on:select={() => handleSelect(capture)}
						selected={selected === capture.id}
					>
						<svelte:fragment slot="start">
							<div class="center-center">
								<svelte:component this={captureIcon} size={12} />
							</div>
						</svelte:fragment>
					</SchemaPickerRow>
				{/each}
			</tbody>
		</DataTable>
	{/if}
</div>
