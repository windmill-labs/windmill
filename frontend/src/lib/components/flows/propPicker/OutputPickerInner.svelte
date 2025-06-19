<script module>
	function useResizeObserver(node, callback) {
		const observer = new ResizeObserver(() => {
			callback()
		})

		observer.observe(node)

		return {
			destroy() {
				observer.disconnect()
			}
		}
	}
</script>

<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Pin, History, Pen, Check, X, Loader2, Pencil } from 'lucide-svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import StepHistory from './StepHistory.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { untrack } from 'svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import type { Job } from '$lib/gen'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import OutputBadge from './OutputBadge.svelte'
	import { twMerge } from 'tailwind-merge'
	import DisplayResultControlBar from '$lib/components/DisplayResultControlBar.svelte'
	import { base } from '$lib/base'

	interface Props {
		prefix?: string
		allowCopy?: boolean
		connectingData?: any | undefined
		mock?:
			| {
					enabled?: boolean
					return_value?: unknown
			  }
			| undefined
		moduleId?: string
		fullResult?: boolean
		closeOnOutsideClick?: boolean
		getLogs?: boolean
		selectedJob?: SelectedJob
		forceJson?: boolean
		isLoading?: boolean
		preview?: 'mock' | 'job' | undefined
		hideHeaderBar?: boolean
		simpleViewer?: any | undefined
		path?: string
		loopStatus?: { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' } | undefined
		customHeight?: number | undefined
		rightMargin?: boolean
		disableMock?: boolean
		disableHistory?: boolean
		lastJob?: SelectedJob
		derivedHistoryOpen?: boolean // derived from historyOpen
		historyOffset?: any
		clazz?: string
		copilot_fix?: import('svelte').Snippet
		onSelect?: (key: string) => void
		onUpdateMock?: (mock: { enabled: boolean; return_value?: unknown }) => void
		onEditInput?: (moduleId: string, key: string) => void
		selectionId?: string
	}

	let {
		lastJob = undefined,
		prefix = '',
		allowCopy = false,
		connectingData = undefined,
		mock = $bindable({ enabled: false }),
		moduleId = '',
		fullResult = false,
		closeOnOutsideClick = false,
		getLogs = false,
		selectedJob = $bindable(undefined),
		forceJson = $bindable(false),
		isLoading = $bindable(false),
		preview = $bindable(undefined),
		hideHeaderBar = false,
		simpleViewer = undefined,
		path = '',
		loopStatus = undefined,
		customHeight = undefined,
		rightMargin = false,
		disableMock = false,
		disableHistory = false,
		derivedHistoryOpen = $bindable(false),
		historyOffset = { mainAxis: 8, crossAxis: -4.5 },
		clazz,
		copilot_fix,
		onSelect,
		onUpdateMock,
		onEditInput,
		selectionId
	}: Props = $props()

	type SelectedJob =
		| ((
				| Job
				| {
						id: string
						result: unknown
						type: 'CompletedJob'
						workspace_id: string
						success: boolean
				  }
		  ) & { preview?: boolean })
		| undefined

	let jsonView = $state(false)
	let clientHeight: number = $state(0)
	let tmpMock: { enabled: boolean; return_value?: unknown } | undefined = $state(undefined)
	let error = $state('')
	let stepHistoryPopover: Popover | undefined = $state(undefined)
	let historyOpen = $state(false)
	let contentEl: HTMLDivElement | undefined = $state(undefined)
	let hasOverflow = $state(false)

	function checkOverflow() {
		if (contentEl) {
			hasOverflow = contentEl.scrollHeight > contentEl.clientHeight
		}
	}

	function selectJob(job: SelectedJob | undefined) {
		if (job && 'result' in job) {
			selectedJob = job
		} else if (lastJob && 'result' in lastJob) {
			selectedJob = lastJob
		}
	}

	$effect(() => {
		if (!lastJob || !('result' in lastJob)) {
			return
		}
		selectJob(lastJob)

		if (lastJob.preview && mock?.enabled) {
			preview = 'job'
			lastJob.preview = false
		}
	})

	function togglePreview(nPrev: 'mock' | 'job' | undefined) {
		if (!nPrev && preview === 'job') {
			// Reset the job
			selectJob(undefined)
		}
		preview = nPrev
	}

	let dblClickDisabled = $state(false)
	let hoveringResult = $state(false)
	let debouncedCanEditWithDblClick = $state(false)
	let debounceTimeout: ReturnType<typeof setTimeout> | null = null
	let canEditWithDblClick = $state(false)
	let displayResultJob: DisplayResult | undefined = $state(undefined)
	let displayResultMock: DisplayResult | undefined = $state(undefined)
	let toolbarLocationJob: 'self' | 'external' | undefined = $state(undefined)
	let toolbarLocationMock: 'self' | 'external' | undefined = $state(undefined)

	function updateCanEditWithDblClick(newValue: boolean) {
		canEditWithDblClick = newValue
		if (debounceTimeout) {
			clearTimeout(debounceTimeout)
		}
		debounceTimeout = setTimeout(() => {
			debouncedCanEditWithDblClick = newValue
		}, 200)
	}

	function togglePin() {
		if (mock?.enabled && !preview) {
			// Unpin
			onUpdateMock?.({
				...mock,
				enabled: false
			})
		} else if (preview === 'mock') {
			// Restore the pin
			onUpdateMock?.({
				...mock,
				enabled: true
			})
		} else if (selectedJob && 'result' in selectedJob) {
			// Pin the job
			let mockValue: any = structuredClone($state.snapshot(selectedJob.result))
			if (selectedJob.result === 'never tested this far') {
				mockValue = { example: 'value' }
			}
			const newMock = {
				enabled: true,
				return_value: mockValue
			}
			onUpdateMock?.(newMock)
		} else {
			// Fallback to mock
			onUpdateMock?.({
				enabled: true,
				return_value: { example: 'value' }
			})
		}
		if (preview) {
			// Reset the preview
			if (historyOpen) {
				stepHistoryPopover?.close()
			}
			togglePreview(undefined)
		}
	}

	let mockUpdateStatus = $derived(
		preview === 'mock' && !mock?.enabled
			? 'restore'
			: preview === 'job' && mock?.enabled && selectedJob?.type === 'CompletedJob'
				? 'override'
				: undefined
	)
	$effect(() => {
		derivedHistoryOpen = historyOpen
	})
	$effect(() => {
		if (displayResultJob && typeof displayResultJob.getToolbarLocation === 'function') {
			toolbarLocationJob = displayResultJob.getToolbarLocation()
		}
	})
	$effect(() => {
		if (displayResultMock && typeof displayResultMock.getToolbarLocation === 'function') {
			toolbarLocationMock = displayResultMock.getToolbarLocation()
		}
	})
	$effect(() => {
		const newValue =
			!!mock?.enabled &&
			!connectingData &&
			!dblClickDisabled &&
			hoveringResult &&
			!jsonView &&
			!preview
		if (newValue != canEditWithDblClick) {
			canEditWithDblClick = newValue
			untrack(() => updateCanEditWithDblClick(newValue))
		}
	})

	let popoverHeight = $derived(customHeight ?? (clientHeight > 0 ? clientHeight : 0))

	const copilot_fix_render = $derived(copilot_fix)
</script>

<div
	class={twMerge('w-full h-full flex flex-col', clazz)}
	bind:clientHeight
	style={canEditWithDblClick ? 'cursor: text;' : ''}
>
	<div
		class={twMerge(
			'text-xs px-1',
			'border-none',
			hideHeaderBar || connectingData ? 'hidden' : 'block',
			hasOverflow ? 'shadow-sm' : ''
		)}
	>
		<div
			class={twMerge(
				'flex flex-row items-center gap-2 justify-between min-h-[33.5px]',
				rightMargin ? `mr-[30px]` : ''
			)}
		>
			<div class="flex flex-row items-center gap-0.5">
				{#if !disableHistory}
					<Popover
						bind:this={stepHistoryPopover}
						floatingConfig={{
							strategy: 'fixed',
							placement: 'left-start',
							offset: historyOffset,
							gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
						}}
						contentClasses="w-[225px] overflow-hidden"
						{closeOnOutsideClick}
						usePointerDownOutside={closeOnOutsideClick}
						disablePopup={!!connectingData || jsonView}
						bind:isOpen={historyOpen}
					>
						{#snippet trigger()}
							<Button
								color="light"
								size="xs2"
								variant="contained"
								btnClasses="bg-surface h-[27px]"
								startIcon={{ icon: History }}
								nonCaptureEvent
							/>
						{/snippet}
						{#snippet content()}
							<div class="rounded-[inherit]" style={`height: ${popoverHeight}px`}>
								<StepHistory
									{moduleId}
									{getLogs}
									on:select={async ({ detail }) => {
										if (!detail) {
											togglePreview(undefined)
											return
										}
										if (detail === 'mock') {
											if (mock?.enabled) {
												togglePreview(undefined)
												return
											}
											togglePreview('mock')
											return
										}
										if (detail.id === lastJob?.id && !mock?.enabled) {
											togglePreview(undefined)
											return
										}

										// Create a timeout to show loading state after 200ms
										const loadingTimeout = setTimeout(() => {
											isLoading = true
										}, 200)

										try {
											const fullJob = await detail.getFullJob()
											if (fullJob) {
												selectJob(fullJob)
												togglePreview('job')
											}
										} finally {
											// Clear the timeout if operation completed before 200ms
											clearTimeout(loadingTimeout)
											isLoading = false
										}
									}}
									mockValue={mock?.return_value}
									mockEnabled={mock?.enabled}
									{path}
									noHistory={loopStatus
										? loopStatus.type === 'self'
											? 'isLoop'
											: 'isInsideLoop'
										: undefined}
								/>
							</div>
						{/snippet}
					</Popover>
				{/if}
				{#if !isLoading}
					<div
						class={twMerge(
							'w-grow min-w-0 flex gap-1 items-center h-[27px] rounded-md  group',
							preview || selectedJob?.id !== lastJob?.id ? 'p-[2px] bg-surface-secondary' : ''
						)}
					>
						{#if loopStatus?.type === 'self'}
							<div
								class={twMerge(
									'min-w-16 rounded-md py-1 px-2 justify-center',
									mockUpdateStatus ? 'bg-surface h-[23px]' : 'bg-surface-secondary h-[27px]'
								)}
							>
								<span
									class="text-xs text-secondary truncate"
									title={loopStatus.flow === 'forloopflow'
										? 'For loop result'
										: 'While loop result'}
								>
									{loopStatus.flow === 'forloopflow' ? 'For loop result' : 'While loop result'}
								</span>
							</div>
						{:else if preview === 'mock' || (mock?.enabled && !preview)}
							<div
								class={twMerge(
									'min-w-16 text-secondary flex flex-row w-fit items-center gap-2 rounded-md bg-surface-secondary p-1 px-2',
									mockUpdateStatus ? 'bg-surface h-[23px]' : 'bg-surface-secondary h-[27px]'
								)}
							>
								<Pin size={12} />
								<span class="text-xs text-secondary truncate"
									>{mock?.enabled ? 'Pin' : 'Last pin'}</span
								>
							</div>
						{:else if preview === 'job' || (selectedJob && !preview)}
							<OutputBadge
								job={selectedJob}
								class={twMerge(
									'min-w-16 text-secondary',
									preview || selectedJob?.id !== lastJob?.id ? 'bg-surface shadow-sm h-[23px]' : ''
								)}
							/>
						{/if}

						{#if preview}
							<button
								class="px-1 shrink-0 text-secondary text-xs text-thin"
								onclick={() => {
									if (historyOpen) {
										// closing history popover exits preview
										stepHistoryPopover?.close()
									}
									togglePreview(undefined)
								}}>Exit preview</button
							>
						{/if}
					</div>
				{/if}

				{#if !disableMock && !isLoading}
					<Tooltip disablePopup={mock?.enabled}>
						<Button
							color="light"
							size="xs2"
							variant="contained"
							btnClasses={twMerge(
								'bg-transparent h-[27px]',
								mock?.enabled || mockUpdateStatus === 'override'
									? 'text-white bg-blue-500 hover:text-primary hover:bg-blue-700 hover:text-gray-100'
									: '',
								jsonView ? 'pointer-events-none' : ''
							)}
							startIcon={{ icon: Pin }}
							iconOnly={mockUpdateStatus !== 'override'}
							on:click={() => togglePin()}
						>
							{#if mockUpdateStatus === 'override'}
								Override pin
							{/if}
						</Button>
						{#snippet text()}
							Pin data
						{/snippet}
					</Tooltip>
				{/if}

				{#if jsonView}
					<Button
						size="xs2"
						color="green"
						variant="contained"
						startIcon={{ icon: Check }}
						btnClasses="h-[27px]"
						on:click={() => {
							if (!tmpMock) {
								return
							}
							jsonView = false
							mock = tmpMock
							onUpdateMock?.({
								enabled: tmpMock?.enabled ?? false,
								return_value: tmpMock?.return_value
							})
							tmpMock = undefined
						}}
						disabled={!!error || !tmpMock}
					/>
					<Button
						size="xs2"
						color="red"
						variant="contained"
						startIcon={{ icon: X }}
						btnClasses="h-[27px]"
						on:click={() => {
							jsonView = false
							tmpMock = undefined
						}}
					/>
				{:else if mock?.enabled && !preview}
					<Tooltip disablePopup={mock?.enabled}>
						<Button
							size="xs2"
							color="light"
							variant="contained"
							startIcon={{ icon: Pen }}
							on:click={() => {
								stepHistoryPopover?.close()
								jsonView = true
								tmpMock = undefined
							}}
							disabled={!mock?.enabled || !!connectingData}
							btnClasses={twMerge(
								'transition-all duration-100 h-[27px]',
								debouncedCanEditWithDblClick
									? 'bg-blue-500/10 text-blue-800 dark:text-blue-200'
									: ''
							)}
						/>
						{#snippet text()}
							{'Pin the output to allow editing'}
						{/snippet}
					</Tooltip>
				{/if}
			</div>

			<div class="px-2">
				{#if selectedJob && 'result' in selectedJob && displayResultJob && toolbarLocationJob === 'external'}
					<DisplayResultControlBar
						workspaceId={selectedJob.workspace_id}
						jobId={selectedJob.id}
						{base}
						result={selectedJob.result}
						disableTooltips={false}
						on:open-drawer={() => {
							if (displayResultJob && typeof displayResultJob.openDrawer === 'function') {
								displayResultJob.openDrawer()
							}
						}}
					/>
				{:else if mock?.enabled && displayResultMock && toolbarLocationMock === 'external'}
					<DisplayResultControlBar
						customUi={{ disableDownload: true }}
						{base}
						result={mock.return_value}
						disableTooltips={false}
						on:open-drawer={() => {
							if (displayResultMock && typeof displayResultMock.openDrawer === 'function') {
								displayResultMock.openDrawer()
							}
						}}
					/>
				{/if}
			</div>
		</div>
	</div>

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_mouse_events_have_key_events -->
	<div
		class={twMerge(
			'grow min-h-0 rounded-md w-full pl-2 py-1 pb-2 overflow-auto',
			'transition-all duration-100 outline outline-2 outline-blue-500/0 outline-offset-[-2px]',
			debouncedCanEditWithDblClick ? 'outline outline-2 outline-blue-500/30' : ''
		)}
		style="scrollbar-gutter: stable;"
	>
		<div
			class={twMerge('h-full w-full rounded-md')}
			bind:this={contentEl}
			onscroll={checkOverflow}
			use:useResizeObserver={checkOverflow}
			onmouseover={(event) => {
				if (
					!event.target ||
					!(event.target instanceof HTMLElement) ||
					event.target.closest('[data-interactive]') ||
					event.target.closest('button') ||
					event.target.closest('input') ||
					event.target.closest('textarea') ||
					event.target.closest('select') ||
					event.target.closest('option') ||
					event.target.closest('label') ||
					event.target.closest('a') ||
					event.target.closest('svg')
				) {
					dblClickDisabled = true
				} else {
					dblClickDisabled = false
				}
			}}
			ondblclick={() => {
				if (canEditWithDblClick) {
					stepHistoryPopover?.close()
					jsonView = true
					tmpMock = undefined
				}
			}}
			onmouseenter={() => {
				hoveringResult = true
			}}
			onmouseleave={() => {
				hoveringResult = false
			}}
		>
			{#if isLoading}
				<div class="flex flex-col items-center justify-center">
					<Loader2 class="animate-spin" />
				</div>
			{:else if connectingData !== undefined || simpleViewer}
				<ObjectViewer
					json={moduleId
						? {
								[moduleId]: connectingData ?? simpleViewer
							}
						: (connectingData ?? simpleViewer)}
					topBrackets={false}
					pureViewer={false}
					{prefix}
					on:select={(e) => {
						onSelect?.(e.detail)
					}}
					{allowCopy}
					{editKey}
				/>
			{:else if jsonView}
				{#await import('$lib/components/JsonEditor.svelte')}
					<Loader2 class="animate-spin" />
				{:then Module}
					<Module.default
						bind:error
						small
						on:changeValue={({ detail }) => {
							if (mock?.enabled) {
								const newMock = {
									enabled: true,
									return_value: structuredClone($state.snapshot(detail))
								}
								tmpMock = newMock
							}
						}}
						code={JSON.stringify(
							mock?.enabled && mock.return_value ? mock.return_value : '',
							null,
							2
						)}
						class="h-full"
					/>
				{/await}
			{:else if (mock?.enabled || preview == 'mock') && preview != 'job'}
				{#if fullResult}
					<div class="break-words relative h-full">
						<DisplayResult
							bind:this={displayResultMock}
							bind:forceJson
							workspaceId={undefined}
							jobId={undefined}
							result={mock?.return_value}
							externalToolbarAvailable
							on:toolbar-location-changed={({ detail }) => {
								toolbarLocationMock = detail
							}}
						/>
					</div>
				{:else}
					<ObjectViewer
						json={moduleId
							? {
									[moduleId]: mock?.return_value
								}
							: mock?.return_value}
						topBrackets={false}
						pureViewer={false}
					/>
				{/if}
			{:else if selectedJob != undefined && 'result' in selectedJob}
				{#if fullResult}
					<div class="break-words relative h-full">
						{#key selectedJob}
							<DisplayResult
								bind:this={displayResultJob}
								bind:forceJson
								workspaceId={selectedJob?.workspace_id}
								jobId={selectedJob?.id}
								result={selectedJob?.result}
								externalToolbarAvailable
								on:toolbar-location-changed={({ detail }) => {
									toolbarLocationJob = detail
								}}
							>
								{#snippet copilot_fix()}
									{@render copilot_fix_render?.()}
								{/snippet}
							</DisplayResult>
						{/key}
					</div>
				{:else}
					<ObjectViewer
						json={moduleId
							? {
									[moduleId]: selectedJob.result
								}
							: selectedJob.result}
						topBrackets={false}
						pureViewer={false}
					/>
				{/if}
			{:else if !lastJob}
				<div class="flex flex-col items-center justify-center h-full">
					<p class="text-xs text-secondary">
						Test this step to see results{#if !disableMock}
							{' or'}
							<button
								class="text-blue-500 hover:text-blue-700 underline"
								onclick={() => {
									const newMock = {
										enabled: true,
										return_value: mock?.return_value ?? { example: 'value' }
									}
									onUpdateMock?.(newMock)
								}}>pin data<Pin size={16} class="inline" /></button
							>{:else}.{/if}
					</p>
				</div>
			{/if}
		</div>
	</div>
</div>

{#snippet editKey(key: string)}
	<button
		onclick={() => onEditInput?.(selectionId ?? '', key)}
		class="h-4 w-fit items-center text-gray-300 dark:text-gray-500 hover:text-primary dark:hover:text-primary px-1 rounded-[0.275rem] align-baseline"
	>
		<Pencil size={12} class="-my-1 inline-flex items-center" />
	</button>
{/snippet}

<style>
	.dbl-click-editable {
		cursor: text;
		transition: all 0.2s ease;
	}

	.dbl-click-editable:hover {
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5) inset;
	}
</style>
