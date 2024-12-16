<script lang="ts">
	import Label from '../Label.svelte'
	import { Plus } from 'lucide-svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Button from '../common/button/Button.svelte'
	import { isObject } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { type TriggerKind } from '../triggers'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { Capture } from '$lib/gen'
	import { captureTriggerKindToTriggerKind } from '../triggers'
	import { twMerge } from 'tailwind-merge'
	import SchemaPicker from '../schema/SchemaPicker.svelte'

	export let path: string
	export let hasPreprocessor = false
	export let canHavePreprocessor = false
	export let isFlow = false
	export let captureType: CaptureTriggerKind = 'webhook'
	export let headless = false
	export let addButton = false
	export let hideCapturesWhenEmpty = false
	export let canEdit = false
	export let showAll = false
	export let maxHeight: number | undefined = undefined
	export let shouldRefreshCaptures = false

	let captures: Capture[] = []
	let selectedCaptures: any[] = []
	let testKind: 'preprocessor' | 'main' = 'main'

	$: hasPreprocessor && (testKind = 'preprocessor')

	function filterCaptures(captures: Capture[], captureType: CaptureTriggerKind | 'all') {
		if (showAll) {
			return captures
		}
		return captures.filter((c) => c.trigger_kind === captureType)
	}
	$: selectedCaptures = filterCaptures(captures, captureType)

	let deleteLoading: number | null = null
	async function deleteCapture(id: number) {
		deleteLoading = id
		try {
			await CaptureService.deleteCapture({
				workspace: $workspaceStore!,
				id
			})
			refreshCaptures()
		} finally {
			deleteLoading = null
		}
	}

	const dispatch = createEventDispatcher<{
		openTriggers: {
			kind: TriggerKind
			config: Record<string, any>
		}
		applyArgs: {
			kind: 'main' | 'preprocessor'
			args: Record<string, any> | undefined
		}
		addPreprocessor: null
		updateSchema: {
			schema: any
			redirect: boolean
		}
	}>()

	async function refreshCaptures() {
		captures = await CaptureService.listCaptures({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path
		})
	}

	refreshCaptures()

	$: if (shouldRefreshCaptures) {
		refreshCaptures()
		shouldRefreshCaptures = false
	}
</script>

{#if selectedCaptures.length > 0 || !hideCapturesWhenEmpty}
	<Label
		label="Captures"
		{headless}
		class={twMerge(
			'flex flex-col h-full divide-y gap-1',
			maxHeight ? `max-h-[${maxHeight}px]` : ''
		)}
	>
		<svelte:fragment slot="header">
			{#if addButton && !showAll}
				<Button
					size="xs2"
					color="light"
					variant="contained"
					iconOnly
					startIcon={{ icon: Plus }}
					on:click={() =>
						dispatch('openTriggers', {
							kind: captureTriggerKindToTriggerKind(captureType),
							config: {}
						})}
				/>
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="action">
			{#if canHavePreprocessor}
				<div>
					<ToggleButtonGroup bind:selected={testKind} class="h-full">
						<ToggleButton value="main" label={isFlow ? 'Flow' : 'Main'} small />
						<ToggleButton
							value="preprocessor"
							label="Preprocessor"
							small
							tooltip="When the runnable has a preprocessor, it receives additional information about the request"
						/>
					</ToggleButtonGroup>
				</div>
			{/if}
		</svelte:fragment>
		<div class="flex flex-col gap-1 pt-2 grow overflow-y-auto">
			{#if selectedCaptures.length === 0}
				<div class="text-xs text-secondary">No {captureType} captures yet</div>
			{:else}
				{#each selectedCaptures as capture}
					{@const payload = isObject(capture.payload) ? capture.payload : {}}
					{@const triggerExtra = isObject(capture.trigger_extra) ? capture.trigger_extra : {}}
					{@const payloadData =
						testKind === 'preprocessor'
							? {
									...payload,
									...triggerExtra
							  }
							: payload}
					<SchemaPicker
						date={capture.created_at}
						{payloadData}
						{testKind}
						{isFlow}
						{canEdit}
						deleteLoading={deleteLoading === capture.id}
						{hasPreprocessor}
						on:updateSchema={(e) => {
							dispatch('updateSchema', {
								schema: payloadData,
								redirect: true
							})
						}}
						on:applyArgs
						on:delete={() => {
							deleteCapture(capture.id)
						}}
						on:addPreprocessor
					/>
				{/each}
			{/if}
		</div>
	</Label>
{/if}
