<script lang="ts">
	import Label from '../Label.svelte'
	import { Clipboard, Info, Trash2, Plus } from 'lucide-svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { copyToClipboard } from '$lib/utils'
	import Button from '../common/button/Button.svelte'
	import CustomPopover from '../CustomPopover.svelte'
	import { convert } from '@redocly/json-to-json-schema'
	import SchemaViewer from '../SchemaViewer.svelte'
	import { isObject } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { type TriggerKind } from '../triggers'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { Capture } from '$lib/gen'
	import { captureTriggerKindToTriggerKind } from '../triggers'

	export let path: string
	export let hasPreprocessor = false
	export let canHavePreprocessor = false
	export let isFlow = false
	export let captureType: CaptureTriggerKind = 'webhook'
	export let headless = false
	export let addButton = false
	export let hideCapturesWhenEmpty = false

	let captures: Capture[] = []
	let selectedCaptures: any[] = []
	let testKind: 'preprocessor' | 'main' = 'main'

	$: hasPreprocessor && (testKind = 'preprocessor')

	$: selectedCaptures = captures.filter((c) => c.trigger_kind === captureType)

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
			path
		})
	}
	refreshCaptures()
</script>

{#if selectedCaptures.length > 0 || !hideCapturesWhenEmpty}
	<Label label="Captures" {headless}>
		<svelte:fragment slot="header">
			{#if addButton}
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
					<ToggleButtonGroup bind:selected={testKind}>
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
		<div class="flex flex-col gap-1 mt-2">
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
					{@const schema =
						isFlow && testKind === 'main'
							? { required: [], properties: {}, ...convert(payloadData) }
							: {}}
					<div class="flex flex-row gap-1">
						<div class="text-xs border p-2 rounded-md overflow-auto grow whitespace-nowrap">
							{JSON.stringify(payloadData)}
						</div>
						<Button
							size="xs2"
							color="light"
							variant="border"
							on:click={() => {
								copyToClipboard(JSON.stringify(payloadData))
							}}
							iconOnly
							startIcon={{ icon: Clipboard }}
						/>

						{#if isFlow && testKind === 'main'}
							<CustomPopover>
								<Button
									size="xs"
									color="light"
									variant="border"
									on:click={() => {
										dispatch('updateSchema', { schema, redirect: true })
									}}
									wrapperClasses="h-full"
								>
									Apply schema
								</Button>

								<svelte:fragment slot="overlay">
									{#if schema}
										<div class="min-w-[400px]">
											<SchemaViewer {schema} />
										</div>
									{/if}
								</svelte:fragment>
							</CustomPopover>
						{/if}

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
										<p>You need to add a preprocessor to use preprocessor captures as args</p>
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
					</div>
				{/each}
			{/if}
		</div>
	</Label>
{/if}
