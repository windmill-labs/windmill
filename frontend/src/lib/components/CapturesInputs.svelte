<script lang="ts">
	import { isObject } from '$lib/utils'
	import SchemaPicker from '$lib/components/schema/SchemaPicker.svelte'
	import type { Capture } from '$lib/gen'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Webhook, Route, Unplug, Mail } from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	export let testKind: 'main' | 'preprocessor' = 'main'
	export let hasPreprocessor = false
	export let isFlow = false
	export let flowPath: string | null = null
	export let scriptHash: string | null = null

	let selected: number | undefined = undefined

	let captures: Capture[] = []
	async function refreshCaptures(path: string | null) {
		if (!path?.length) return
		captures = await CaptureService.listCaptures({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path: path ?? ''
		})
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
</script>

<div class="flex flex-col gap-1 grow overflow-y-auto">
	{#if captures.length === 0}
		<div class="text-xs text-secondary">No captures yet</div>
	{:else}
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
			<SchemaPicker
				date={capture.created_at}
				{payloadData}
				{testKind}
				{isFlow}
				canEdit={false}
				deleteLoading={false}
				{hasPreprocessor}
				allowApplyArgs={false}
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
			</SchemaPicker>
		{/each}
	{/if}
</div>
