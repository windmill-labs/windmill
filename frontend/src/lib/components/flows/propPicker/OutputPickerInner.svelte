<script context="module">
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
	import { Pin, History, Pen, Check, X, Loader2 } from 'lucide-svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import StepHistory from './StepHistory.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { createEventDispatcher } from 'svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import type { Job } from '$lib/gen'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import OutputBadge from './OutputBadge.svelte'
	import { twMerge } from 'tailwind-merge'
	import DisplayResultControlBar from '$lib/components/DisplayResultControlBar.svelte'
	import { base } from '$lib/base'

	export let prefix: string = ''
	export let allowCopy: boolean = false
	export let connectingData: any | undefined = undefined
	export let mock:
		| {
				enabled?: boolean
				return_value?: unknown
		  }
		| undefined = { enabled: false }
	export let moduleId: string = ''
	export let fullResult: boolean = false
	export let closeOnOutsideClick: boolean = false
	export let getLogs: boolean = false
	export let selectedJob: SelectedJob = undefined
	export let forceJson: boolean = false
	export let isLoading: boolean = false
	export let preview: 'mock' | 'job' | undefined = undefined
	export let hideHeaderBar: boolean = false
	export let simpleViewer: any | undefined = undefined
	export let path: string = ''
	export let loopStatus:
		| { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' }
		| undefined = undefined
	export let customHeight: number | undefined = undefined
	export let rightMargin: boolean = false
	export let disableMock: boolean = false
	export let disableHistory: boolean = false
	export let derivedHistoryOpen: boolean = false // derived from historyOpen
	export let historyOffset = { mainAxis: 8, crossAxis: -4.5 }

	type SelectedJob =
		| Job
		| {
				id: string
				result: unknown
				type: 'CompletedJob'
				workspace_id: string
				success: boolean
		  }
		| undefined

	const dispatch = createEventDispatcher<{
		updateMock: { enabled: boolean; return_value?: unknown }
	}>()

	let jsonView = false
	let clientHeight: number = 0
	let tmpMock: { enabled: boolean; return_value?: unknown } | undefined = undefined
	let error = ''
	let stepHistoryPopover: Popover | undefined = undefined
	let lastJob: SelectedJob = undefined
	let historyOpen = false
	let contentEl: HTMLDivElement | undefined = undefined
	let hasOverflow = false

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

	export function setLastJob(job: SelectedJob, setPreview: boolean = false) {
		if (!job || !('result' in job)) {
			return
		}
		lastJob = structuredClone(job)
		selectJob(lastJob)
		if (setPreview && mock?.enabled) {
			preview = 'job'
		}
	}

	function togglePreview(nPrev: 'mock' | 'job' | undefined) {
		if (!nPrev && preview === 'job') {
			// Reset the job
			selectJob(undefined)
		}
		preview = nPrev
	}

	$: mockUpdateStatus =
		preview === 'mock' && !mock?.enabled
			? 'restore'
			: preview === 'job' && mock?.enabled && selectedJob?.type === 'CompletedJob'
				? 'override'
				: undefined

	let dblClickDisabled = false
	let hoveringResult = false
	let debouncedCanEditWithDblClick = false
	let debounceTimeout: ReturnType<typeof setTimeout> | null = null
	let canEditWithDblClick = false
	let displayResultJob: DisplayResult | undefined = undefined
	let displayResultMock: DisplayResult | undefined = undefined
	let toolbarLocationJob: 'self' | 'external' | undefined = undefined
	let toolbarLocationMock: 'self' | 'external' | undefined = undefined

	$: derivedHistoryOpen = historyOpen

	$: if (displayResultJob && typeof displayResultJob.getToolbarLocation === 'function') {
		toolbarLocationJob = displayResultJob.getToolbarLocation()
	}

	$: if (displayResultMock && typeof displayResultMock.getToolbarLocation === 'function') {
		toolbarLocationMock = displayResultMock.getToolbarLocation()
	}

	$: {
		const newValue =
			!!mock?.enabled &&
			!connectingData &&
			!dblClickDisabled &&
			hoveringResult &&
			!jsonView &&
			!preview
		if (newValue != canEditWithDblClick) {
			canEditWithDblClick = newValue
			updateCanEditWithDblClick(newValue)
		}
	}

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
			dispatch('updateMock', {
				...mock,
				enabled: false
			})
		} else if (preview === 'mock') {
			// Restore the pin
			dispatch('updateMock', {
				...mock,
				enabled: true
			})
		} else if (selectedJob && 'result' in selectedJob) {
			// Pin the job
			let mockValue = structuredClone(selectedJob.result)
			if (selectedJob.result === 'never tested this far') {
				mockValue = { example: 'value' }
			}
			const newMock = {
				enabled: true,
				return_value: mockValue
			}
			dispatch('updateMock', newMock)
		} else {
			// Fallback to mock
			dispatch('updateMock', {
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

	$: popoverHeight = customHeight ?? (clientHeight > 0 ? clientHeight : 0)
</script>

<div
	class={twMerge('w-full h-full flex flex-col', $$props.class)}
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
						<svelte:fragment slot="trigger">
							<Button
								color="light"
								size="xs2"
								variant="contained"
								btnClasses="bg-surface h-[27px]"
								startIcon={{ icon: History }}
								nonCaptureEvent
							/>
						</svelte:fragment>
						<svelte:fragment slot="content">
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
						</svelte:fragment>
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
								on:click={() => {
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
						<svelte:fragment slot="text">Pin data</svelte:fragment>
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
							dispatch('updateMock', {
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
						<svelte:fragment slot="text">
							{'Pin the output to allow editing'}
						</svelte:fragment>
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

	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-mouse-events-have-key-events -->
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
			on:scroll={checkOverflow}
			use:useResizeObserver={checkOverflow}
			on:mouseover={(event) => {
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
			on:dblclick={() => {
				if (canEditWithDblClick) {
					stepHistoryPopover?.close()
					jsonView = true
					tmpMock = undefined
				}
			}}
			on:mouseenter={() => {
				hoveringResult = true
			}}
			on:mouseleave={() => {
				hoveringResult = false
			}}
		>
			{#if isLoading}
				<div class="flex flex-col items-center justify-center">
					<Loader2 class="animate-spin" />
				</div>
			{:else if connectingData || simpleViewer}
				<ObjectViewer
					json={moduleId
						? {
								[moduleId]: connectingData ?? simpleViewer
							}
						: (connectingData ?? simpleViewer)}
					topBrackets={false}
					pureViewer={false}
					{prefix}
					on:select
					{allowCopy}
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
									return_value: structuredClone(detail)
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
								<svelte:fragment slot="copilot-fix">
									<slot name="copilot-fix" />
								</svelte:fragment>
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
								on:click={() => {
									const newMock = {
										enabled: true,
										return_value: mock?.return_value ?? { example: 'value' }
									}
									dispatch('updateMock', newMock)
								}}>pin data<Pin size={16} class="inline" /></button
							>{:else}.{/if}
					</p>
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.dbl-click-editable {
		cursor: text;
		transition: all 0.2s ease;
	}

	.dbl-click-editable:hover {
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5) inset;
	}
</style>
