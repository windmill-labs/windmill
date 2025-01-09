<script lang="ts">
	import Label from '../Label.svelte'
	import { Info, Trash2 } from 'lucide-svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Button from '../common/button/Button.svelte'
	import CustomPopover from '../CustomPopover.svelte'
	import { Webhook, Route, Unplug, Mail, Play } from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import { isObject } from '$lib/utils'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { type TriggerKind } from '../triggers'
	import { CaptureService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { type CaptureTriggerKind } from '$lib/gen'
	import type { Capture } from '$lib/gen'
	import CaptureButton from '$lib/components/triggers/CaptureButton.svelte'
	import { DataTable } from '../table/index'
	import { sendUserToast } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import SchemaPickerRow from '$lib/components/schema/SchemaPickerRow.svelte'
	import { clickOutside } from '$lib/utils'

	export let path: string
	export let hasPreprocessor = false
	export let canHavePreprocessor = false
	export let isFlow = false
	export let captureType: CaptureTriggerKind | undefined = undefined
	export let headless = false
	export let addButton = false
	export let hideCapturesWhenEmpty = false
	export let canEdit = false
	export let captureActive = false

	let captures: Capture[] = []
	let selected: number | undefined = undefined
	let hovered: number | undefined = undefined
	let testKind: 'preprocessor' | 'main' = 'main'
	let hasMore = false
	let page = 1
	let perPage = 10
	$: hasPreprocessor && (testKind = 'preprocessor')

	let deleteLoading: number | null = null
	async function deleteCapture(id: number) {
		deleteLoading = id
		try {
			await CaptureService.deleteCapture({
				workspace: $workspaceStore!,
				id
			})
			loadCaptures(true)
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
			payloadData: any
			redirect: boolean
			args?: boolean
		}
		select: any
		testWithArgs: any
	}>()

	let hasAlreadyFailed = false
	export async function loadCaptures(refresh?: boolean) {
		hasMore = false

		try {
			const newCaptures = await CaptureService.listCaptures({
				workspace: $workspaceStore!,
				runnableKind: isFlow ? 'flow' : 'script',
				path: path ?? '',
				triggerKind: captureType,
				page: refresh ? 1 : page,
				perPage: refresh ? Math.max(page - 1, 1) * perPage : perPage
			})
			if (
				newCaptures.length === 0 ||
				(newCaptures.length === captures.length &&
					newCaptures.every((capture, i) => capture.id === captures[i].id))
			) {
				return
			} else {
				if (refresh) {
					captures = newCaptures
					page = 1
				} else {
					captures = [...captures, ...newCaptures]
				}
				hasMore = captures.length === perPage * page
				page++
			}
		} catch (error) {
			console.error('Failed to load captures:', error)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			sendUserToast(`Failed to load captures: ${error}`, true)
			hasMore = captures ? captures.length === perPage * page : false
		}
	}

	loadCaptures(true)

	function handleSelect(capture: Capture) {
		if (selected === capture.id) {
			deselect()
		} else {
			selected = capture.id
			dispatch('select', capture.payload)
		}
	}

	function deselect() {
		selected = undefined
		dispatch('select', undefined)
	}

	onDestroy(() => {
		deselect()
	})

	const captureKindToIcon: Record<string, any> = {
		webhook: Webhook,
		http: Route,
		email: Mail,
		websocket: Unplug,
		kafka: KafkaIcon
	}

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-schema-picker], [data-schema-picker] *')
		) as HTMLElement[]
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selected) {
			deselect()
		}
	}

	let firstClick = true
</script>

<svelte:window on:keydown={handleKeydown} />

{#if captures.length > 0 || !hideCapturesWhenEmpty}
	<Label label="Trigger captures" {headless} class="h-full flex flex-col gap-1">
		<svelte:fragment slot="header">
			{#if addButton}
				<div class="inline-block">
					<CaptureButton small={true} on:openTriggers />
				</div>
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="action">
			{#if canHavePreprocessor && captures.length > 0}
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

		<div
			class={twMerge('grow min-h-0', captures.length > 7 ? 'h-[300px]' : 'h-auto')}
			use:clickOutside={{ capture: false, exclude: getPropPickerElements }}
			on:click_outside={() => {
				if (firstClick) {
					firstClick = false
					return
				}
				deselect()
			}}
		>
			{#if captures.length === 0}
				<div class="text-xs text-secondary text-center">
					{`No ${captureType ?? 'trigger'} tests yet`}
				</div>
			{:else}
				<DataTable
					size="xs"
					on:loadMore={() => {
						loadCaptures()
					}}
					{hasMore}
					tableFixed={true}
					infiniteScroll
				>
					<colgroup>
						<col class="w-8" />
						<col class="w-20" />
						<col />
					</colgroup>

					<tbody class="w-full">
						{#each captures as capture}
							{@const payload = isObject(capture.payload) ? capture.payload : {}}
							{@const triggerExtra = isObject(capture.trigger_extra) ? capture.trigger_extra : {}}
							{@const payloadData =
								testKind === 'preprocessor'
									? {
											...payload,
											...triggerExtra
									  }
									: { ...payload }}
							{@const captureIcon = captureKindToIcon[capture.trigger_kind]}
							<SchemaPickerRow
								date={capture.created_at}
								{payloadData}
								on:updateSchema
								on:addPreprocessor
								on:select={() => handleSelect(capture)}
								selected={selected === capture.id}
								on:hover={(e) => (hovered = e.detail ? capture.id : undefined)}
							>
								<svelte:fragment slot="start">
									<div class="center-center">
										<svelte:component this={captureIcon} size={12} />
									</div>
								</svelte:fragment>

								<svelte:fragment slot="extra">
									{#if canEdit}
										<div class="flex flex-row items-center gap-2 px-2">
											{#if testKind === 'preprocessor' && !hasPreprocessor}
												<CustomPopover noPadding>
													<Button
														size="xs2"
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
															<p>
																You need to add a preprocessor to use preprocessor captures as args
															</p>
															<Button
																size="xs2"
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
													size="xs2"
													color={hovered === capture.id || selected === capture.id
														? 'dark'
														: 'light'}
													dropdownItems={[
														{
															label: 'Use as input schema',
															onClick: () => {
																dispatch('updateSchema', {
																	payloadData,
																	redirect: true,
																	args: true
																})
															},
															disabled: !isFlow || testKind !== 'main'
														}
													].filter((item) => !item.disabled)}
													on:click={() => {
														if (isFlow && testKind === 'main') {
															dispatch('testWithArgs', payloadData)
														} else {
															dispatch('applyArgs', {
																kind: testKind,
																args: payloadData
															})
														}
													}}
													disabled={testKind === 'preprocessor' && !hasPreprocessor}
													title={isFlow && testKind === 'main'
														? 'Test flow with args'
														: 'Apply args to preprocessor'}
													startIcon={isFlow && testKind === 'main' ? { icon: Play } : {}}
												>
													{isFlow && testKind === 'main' ? 'Test' : 'Apply args'}
												</Button>
											{/if}
										</div>

										<Button
											size="xs2"
											color="red"
											variant="border"
											iconOnly
											startIcon={{ icon: Trash2 }}
											disabled={captureActive}
											loading={deleteLoading === capture.id}
											on:click={() => {
												deleteCapture(capture.id)
											}}
											btnClasses="border-0"
										/>
									{/if}
								</svelte:fragment>
							</SchemaPickerRow>
						{/each}
					</tbody>
				</DataTable>
			{/if}
		</div>
	</Label>
{/if}
