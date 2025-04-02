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
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import StepHistory from './StepHistory.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { createEventDispatcher } from 'svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import type { Job } from '$lib/gen'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import OutputBadge from './OutputBadge.svelte'
	import { classes } from '$lib/components/common/alert/model'
	import { twMerge } from 'tailwind-merge'

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
	let stepHistory: StepHistory | undefined = undefined
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
		if (!job || !('result' in job)) {
			if (lastJob && 'result' in lastJob) {
				selectJob(lastJob)
			}
			return
		}
		selectedJob = job
	}

	export function setLastJob(job: SelectedJob, setPreview: boolean = false) {
		if (!job || !('result' in job)) {
			return
		}
		lastJob = structuredClone(job)
		selectJob(lastJob)
		if (setPreview) {
			togglePreview('job')
		}
	}

	function togglePreview(nPrev: 'mock' | 'job' | undefined) {
		if (nPrev === 'mock') {
			preview = 'mock'
		} else if (nPrev === 'job' && mock?.enabled && selectedJob) {
			preview = 'job'
		} else {
			preview = undefined
		}
	}

	$: infoMessage =
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

	$: {
		const newValue =
			!!mock?.enabled && !connectingData && !dblClickDisabled && hoveringResult && !jsonView
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

	$: popoverHeight = customHeight ?? (clientHeight > 0 ? clientHeight : 0)
</script>

<div
	class="w-full h-full flex flex-col"
	bind:clientHeight
	style={canEditWithDblClick ? 'cursor: text;' : ''}
	id="result-container"
>
	<div
		class={twMerge(
			infoMessage ? `${classes['info'].descriptionClass} ${classes['info'].bgClass}` : '',
			'text-xs px-1',
			'border-none',
			hideHeaderBar || connectingData ? 'hidden' : 'block',
			hasOverflow ? 'shadow-sm' : ''
		)}
	>
		<div class="flex flex-row items-center gap-0.5 min-h-[33.5px] mr-[30px]">
			<Popover
				bind:this={stepHistoryPopover}
				floatingConfig={{
					placement: 'left-start',
					offset: { mainAxis: 10, crossAxis: -6 },
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
						btnClasses="bg-surface"
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
									selectJob(undefined)
									togglePreview(undefined)
									return
								}
								if (detail === 'mock') {
									if (mock?.enabled) {
										selectJob(undefined)
										togglePreview(undefined)
										return
									}
									togglePreview('mock')
									return
								}
								isLoading = true
								const fullJob = await detail.getFullJob()
								if (fullJob) {
									selectJob(fullJob)
									togglePreview('job')
								}
								isLoading = false
							}}
							mockValue={mock?.return_value}
							mockEnabled={mock?.enabled}
							bind:this={stepHistory}
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
			{#if infoMessage === 'override'}
				<span>
					<Pin size={14} class="inline" />
					This step is pinned.
					<button
						class="inline-block text-xs underline"
						on:click={() => {
							if (historyOpen) {
								stepHistory?.deselect()
							} else {
								selectJob(lastJob)
								togglePreview(undefined)
							}
						}}
					>
						See pin</button
					>
					or
					<button
						class="inline-block text-xs underline"
						on:click={() => {
							const newMock = {
								enabled: true,
								return_value: selectedJob && 'result' in selectedJob ? selectedJob.result : ''
							}
							dispatch('updateMock', newMock)
							if (historyOpen) {
								stepHistory?.deselect()
								stepHistoryPopover?.close()
							} else {
								selectJob(lastJob)
								togglePreview(undefined)
							}
						}}
					>
						override pin
					</button>
				</span>
			{:else if infoMessage === 'restore'}
				<span>
					<button
						class="inline-block text-xs px-2 py-1 underline"
						on:click={() => {
							dispatch('updateMock', {
								...mock,
								enabled: true
							})
							selectJob(undefined) // reset the job
							togglePreview(undefined)
							stepHistoryPopover?.close()
						}}
					>
						Restore pin <Pin size={14} class="inline" />
					</button>
					or
					<button
						class="inline-block text-xs px-2 py-1 underline"
						on:click={() => {
							selectJob(lastJob)
							togglePreview(undefined)
						}}
					>
						See last result
					</button>
				</span>
			{:else}
				<Button
					color="light"
					size="xs2"
					variant="contained"
					btnClasses={twMerge(
						'bg-transparent',
						mock?.enabled
							? 'text-white bg-blue-500 hover:text-primary hover:bg-blue-700 hover:text-gray-100'
							: '',
						jsonView ? 'pointer-events-none' : ''
					)}
					startIcon={{ icon: Pin }}
					iconOnly
					on:click={() => {
						if (mock?.enabled) {
							dispatch('updateMock', {
								...mock,
								enabled: false
							})
						} else {
							if (selectedJob && 'result' in selectedJob) {
								let mockValue = structuredClone(selectedJob.result)
								if (selectedJob.result === 'never tested this far') {
									mockValue = { example: 'value' }
								}
								const newMock = {
									enabled: true,
									return_value: mockValue
								}
								dispatch('updateMock', newMock)
								selectJob(undefined) // reset the job
							} else {
								if (mock?.return_value) {
									dispatch('updateMock', {
										enabled: true,
										return_value: mock.return_value
									})
								} else {
									dispatch('updateMock', {
										enabled: true,
										return_value: { example: 'value' }
									})
								}
							}
						}
					}}
				/>

				{#if jsonView}
					<Button
						size="xs2"
						color="green"
						variant="contained"
						startIcon={{ icon: Check }}
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
						on:click={() => {
							jsonView = false
							tmpMock = undefined
						}}
					/>
				{:else}
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
								'transition-all duration-100 ',
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
			{/if}
			{#if !isLoading && selectedJob && !preview && !mock?.enabled && 'result' in selectedJob}
				<div class="w-grow min-w-0 flex gap-1 items-center">
					{#if loopStatus?.type === 'self'}
						<div class="min-w-16 rounded-md bg-surface-secondary py-1 px-2 justify-center">
							<span class="text-xs text-secondary">
								{loopStatus.flow === 'forloopflow' ? 'For loop result' : 'While loop result'}
							</span>
						</div>
					{:else}
						<OutputBadge job={selectedJob} class="grow min-w-16" />
					{/if}
					{#if selectedJob.id !== lastJob?.id}
						<button
							class="px-1 shrink-0 underline"
							on:click={() => {
								if (historyOpen) {
									stepHistory?.deselect()
								} else {
									selectJob(lastJob)
									togglePreview(undefined)
								}
							}}>See last result</button
						>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-mouse-events-have-key-events -->
	<div
		class={twMerge(
			'grow min-h-0 rounded-md w-full py-1 pb-2 overflow-auto',
			'transition-all duration-200 outline outline-2 outline-blue-500/0 outline-offset-[-2px]',
			debouncedCanEditWithDblClick ? 'outline outline-2 outline-blue-500/30' : ''
		)}
		style="scrollbar-gutter: stable both-edges;"
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
						: connectingData ?? simpleViewer}
					topBrackets={false}
					pureViewer={false}
					{prefix}
					on:select
					{allowCopy}
				/>
			{:else if jsonView}
				<JsonEditor
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
			{:else if (mock?.enabled || preview == 'mock') && preview != 'job'}
				{#if fullResult}
					<div class="break-words relative h-full">
						<DisplayResult
							bind:forceJson
							workspaceId={undefined}
							jobId={undefined}
							result={mock?.return_value}
							portal="#result-container"
							useFloatingControls
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
								bind:forceJson
								workspaceId={selectedJob?.workspace_id}
								jobId={selectedJob?.id}
								result={selectedJob?.result}
								portal="#result-container"
								useFloatingControls
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
						Test this step to see results or <button
							class="text-blue-500 hover:text-blue-700 underline"
							on:click={() => {
								const newMock = {
									enabled: true,
									return_value: mock?.return_value ?? { example: 'value' }
								}
								dispatch('updateMock', newMock)
							}}>pin data<Pin size={16} class="inline" /></button
						>
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
