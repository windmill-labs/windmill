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
	import CaptureButton from '$lib/components/triggers/CaptureButton.svelte'
	import InfiniteList from '../InfiniteList.svelte'
	import { sendUserToast } from '$lib/utils'
	import SchemaPickerRow from '$lib/components/schema/SchemaPickerRow.svelte'
	import { clickOutside } from '$lib/utils'

	export let path: string
	export let hasPreprocessor = false
	export let canHavePreprocessor = false
	export let isFlow = false
	export let captureType: CaptureTriggerKind | undefined = undefined
	export let headless = false
	export let addButton = false
	export let canEdit = false
	export let fullHeight = true

	let selected: number | undefined = undefined
	let testKind: 'preprocessor' | 'main' = 'main'
	let isEmpty: boolean = true
	let infiniteList: InfiniteList | null = null
	let firstClick = true
	let capturesLength = 0

	$: hasPreprocessor && (testKind = 'preprocessor')

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

	export async function loadCaptures(refresh: boolean = false) {
		await infiniteList?.loadData(refresh ? 'refresh' : 'loadMore')
	}

	function initLoadCaptures(testKind: 'preprocessor' | 'main' = 'main') {
		const loadInputsPageFn = async (page: number, perPage: number) => {
			const captures = await CaptureService.listCaptures({
				workspace: $workspaceStore!,
				runnableKind: isFlow ? 'flow' : 'script',
				path: path ?? '',
				triggerKind: captureType,
				page,
				perPage
			})
			let capturesWithPayload = captures.map((capture) => {
				const payload = isObject(capture.payload) ? capture.payload : {}
				const triggerExtra = isObject(capture.trigger_extra) ? capture.trigger_extra : {}
				return {
					...capture,
					payloadData:
						testKind === 'preprocessor'
							? {
									...payload,
									...triggerExtra
							  }
							: { ...payload }
				}
			})
			return capturesWithPayload
		}
		infiniteList?.setLoader(loadInputsPageFn)

		const deleteInputFn = async (id: any) => {
			await CaptureService.deleteCapture({
				workspace: $workspaceStore!,
				id
			})
		}
		infiniteList?.setDeleteItemFn(deleteInputFn)
	}

	function handleSelect(capture: any) {
		if (selected === capture.id) {
			deselect()
		} else {
			selected = capture.id
			dispatch('select', capture.payloadData)
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

	function handleError(error: { type: string; error: any }) {
		if (error.type === 'delete') {
			sendUserToast(`Failed to delete capture: ${error.error}`, true)
		} else if (error.type === 'load') {
			sendUserToast(`Failed to load captures: ${error.error}`, true)
		}
	}

	$: path && infiniteList && initLoadCaptures()
</script>

<svelte:window on:keydown={handleKeydown} />

<Label label="Trigger captures" {headless} class="h-full flex flex-col gap-1">
	<svelte:fragment slot="header">
		{#if addButton}
			<div class="inline-block">
				<CaptureButton small={true} on:openTriggers />
			</div>
		{/if}
	</svelte:fragment>
	<svelte:fragment slot="action">
		{#if canHavePreprocessor && !isEmpty}
			<div>
				<ToggleButtonGroup
					bind:selected={testKind}
					class="h-full"
					on:selected={(e) => {
						initLoadCaptures(e.detail)
					}}
				>
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
		class={fullHeight ? 'h-full' : capturesLength > 7 ? 'h-[300px]' : 'h-fit'}
		use:clickOutside={{ capture: false, exclude: getPropPickerElements }}
		on:click_outside={() => {
			if (firstClick) {
				firstClick = false
				return
			}
			deselect()
		}}
	>
		<InfiniteList
			bind:this={infiniteList}
			selectedItemId={selected}
			bind:isEmpty
			on:error={(e) => handleError(e.detail)}
			on:select={(e) => handleSelect(e.detail)}
			bind:length={capturesLength}
		>
			<svelte:fragment slot="columns">
				<colgroup>
					<col class="w-8" />
					<col class="w-20" />
					<col />
				</colgroup>
			</svelte:fragment>
			<svelte:fragment let:item let:hover>
				{@const captureIcon = captureKindToIcon[item.trigger_kind]}
				<SchemaPickerRow
					date={item.created_at}
					payloadData={item.payloadData}
					selected={selected === item.id}
					hovering={hover}
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
												<p> You need to add a preprocessor to use preprocessor captures as args </p>
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
										color={hover || selected === item.id ? 'dark' : 'light'}
										dropdownItems={[
											{
												label: 'Use as input schema',
												onClick: () => {
													dispatch('updateSchema', {
														payloadData: item.payloadData,
														redirect: true,
														args: true
													})
												},
												disabled: !isFlow || testKind !== 'main'
											}
										].filter((item) => !item.disabled)}
										on:click={() => {
											if (isFlow && testKind === 'main') {
												dispatch('testWithArgs', item.payloadData)
											} else {
												dispatch('applyArgs', {
													kind: testKind,
													args: item.payloadData
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
								<Button
									size="xs2"
									color="light"
									variant="contained"
									iconOnly
									startIcon={{ icon: Trash2 }}
									loading={item.isDeleting}
									on:click={() => {
										infiniteList?.deleteItem(item.id)
									}}
									btnClasses="hover:text-white hover:bg-red-500 text-red-500"
								/>
							</div>
						{/if}
					</svelte:fragment>
				</SchemaPickerRow>
			</svelte:fragment>
			<svelte:fragment slot="empty">
				<div class="text-center text-xs text-tertiary">No captures yet</div>
			</svelte:fragment>
		</InfiniteList>
	</div>
</Label>
