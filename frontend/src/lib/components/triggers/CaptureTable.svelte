<script lang="ts">
	import Label from '../Label.svelte'
	import { Info, Trash2, Plus } from 'lucide-svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { copyToClipboard } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import CustomPopover from '../CustomPopover.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { isObject } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { type TriggerKind } from '../triggers'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { Capture } from '$lib/gen'
	import { captureTriggerKindToTriggerKind } from '../triggers'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'

	export let path: string
	export let hasPreprocessor = false
	export let canHavePreprocessor = false
	export let isFlow = false
	export let captureType: CaptureTriggerKind | undefined = undefined
	export let headless = false
	export let addButton = false
	export let hideCapturesWhenEmpty = false
	export let canEdit = false
	export let maxHeight: number | undefined = undefined

	let captures: Capture[] = []
	let testKind: 'preprocessor' | 'main' = 'main'

	$: hasPreprocessor && (testKind = 'preprocessor')

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

	export async function refreshCaptures() {
		captures = await CaptureService.listCaptures({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path,
			triggerKind: captureType
		})
	}
	refreshCaptures()
</script>

{#if captures.length > 0 || !hideCapturesWhenEmpty}
	<Label
		label="Trigger tests"
		{headless}
		class={twMerge(
			'flex flex-col h-full divide-y gap-1',
			maxHeight ? `max-h-[${maxHeight}px]` : ''
		)}
	>
		<svelte:fragment slot="header">
			{#if addButton && captureType !== undefined}
				<Button
					size="xs2"
					color="light"
					variant="contained"
					iconOnly
					startIcon={{ icon: Plus }}
					on:click={() => {
						captureType &&
							dispatch('openTriggers', {
								kind: captureTriggerKindToTriggerKind(captureType),
								config: {}
							})
					}}
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
			{#if captures.length === 0}
				<div class="text-xs text-secondary">
					{`No ${captureType ?? 'trigger'} tests yet`}
				</div>
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
					{@const schema =
						isFlow && testKind === 'main'
							? { required: [], properties: {}, ...convert(payloadData) }
							: {}}
					<div class="flex flex-row gap-1" transition:slide>
						<CustomPopover class="w-full overflow-auto flex items-center justify-center">
							<!-- svelte-ignore a11y-click-events-have-key-events -->
							<!-- svelte-ignore a11y-no-static-element-interactions -->
							<div
								class="text-xs border w-full font-normal text-left p-1 rounded-md whitespace-nowrap overflow-hidden text-ellipsis"
								on:click={() => {
									copyToClipboard(JSON.stringify(payloadData))
								}}
							>
								{JSON.stringify(payloadData)}
							</div>
							<svelte:fragment slot="overlay">
								<ObjectViewer json={payloadData} />
							</svelte:fragment>
						</CustomPopover>

						{#if testKind === 'preprocessor' && !hasPreprocessor}
							<CustomPopover noPadding>
								<Button
									size="xs"
									color="dark"
									disabled
									endIcon={{
										icon: Info
									}}
									wrapperClasses="h-full"
								>
									Apply args
								</Button>
								<svelte:fragment slot="overlay">
									<div class="text-sm p-2 flex flex-col gap-1 items-start">
										<p> You need to add a preprocessor to use preprocessor captures as args </p>
										<Button
											size="xs"
											color="dark"
											on:click={() => {
												dispatch('addPreprocessor')
											}}
										>
											Add preprocessor
										</Button>
									</div>
								</svelte:fragment>
							</CustomPopover>
						{:else}
							<Button
								size="xs"
								color="dark"
								dropdownItems={[
									{
										label: 'Apply schema only',
										onClick: () => {
											dispatch('updateSchema', { schema, redirect: true })
										},
										disabled: !isFlow || testKind !== 'main'
									}
								].filter((item) => !item.disabled)}
								on:click={() => {
									if (isFlow && testKind === 'main') {
										dispatch('updateSchema', { schema, redirect: false })
									}
									dispatch('applyArgs', {
										kind: testKind,
										args: payloadData
									})
								}}
								disabled={testKind === 'preprocessor' && !hasPreprocessor}
							>
								{isFlow && testKind === 'main' ? 'Apply schema and args' : 'Apply args'}
							</Button>
						{/if}

						{#if canEdit}
							<Button
								size="xs2"
								color="red"
								iconOnly
								startIcon={{ icon: Trash2 }}
								loading={deleteLoading === capture.id}
								on:click={() => {
									deleteCapture(capture.id)
								}}
							/>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	</Label>
{/if}
