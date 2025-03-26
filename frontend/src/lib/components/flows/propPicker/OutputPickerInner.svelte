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
	let jsonData: any = undefined

	function selectJob(job: SelectedJob | undefined) {
		selectedJob = job
		if (!job || !('result' in job)) {
			if (lastJob && 'result' in lastJob) {
				selectJob(lastJob)
			}
			return
		}

		jsonData = job.result
		if (mock?.enabled) {
			const newMock = {
				enabled: true,
				return_value: job.result
			}
			tmpMock = newMock
		}
	}

	function selectMockValue() {
		jsonData = mock?.return_value
		const newMock = {
			enabled: true,
			return_value: mock?.return_value
		}
		tmpMock = newMock
	}

	export function setLastJob(job: SelectedJob, setPreview: boolean = false) {
		if (!job) {
			return
		}
		lastJob = structuredClone(job)
		selectJob(lastJob)
		if (setPreview) {
			preview = 'job'
		}
	}

	$: infoMessage =
		preview === 'mock' && !mock?.enabled
			? 'restore'
			: preview === 'job' && mock?.enabled && selectedJob?.type === 'CompletedJob'
			? 'override'
			: undefined
</script>

<div class="w-full h-full flex flex-col" bind:clientHeight>
	<div
		class={twMerge(
			infoMessage ? `${classes['info'].descriptionClass} ${classes['info'].bgClass}` : '',
			'text-xs px-1',
			'border-none',
			hideHeaderBar || connectingData ? 'hidden' : 'block'
		)}
	>
		<div class="flex flex-row items-center gap-0.5 min-h-[30px] mr-[30px]">
			<Popover
				bind:this={stepHistoryPopover}
				floatingConfig={{
					placement: 'left-start',
					offset: { mainAxis: 10, crossAxis: -4 },
					gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
				}}
				contentClasses="w-[275px] overflow-hidden"
				{closeOnOutsideClick}
				disablePopup={!!connectingData}
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
					<div class="rounded-[inherit]" style={`height: ${clientHeight}px`}>
						<StepHistory
							{moduleId}
							{getLogs}
							on:select={({ detail }) => {
								if (detail === 'mock') {
									if (mock?.enabled) {
										selectJob(undefined)
										preview = undefined
										return
									}
									selectMockValue()
									preview = 'mock'
									return
								}
								selectJob(detail)
								preview = mock?.enabled && detail ? 'job' : undefined
							}}
							mockValue={mock?.return_value}
							mockEnabled={mock?.enabled}
							bind:this={stepHistory}
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
								preview = undefined
							}
						}}
					>
						See pin</button
					>
					or
					<button
						class="inline-block text-xs underline"
						on:click={() => {
							if (!tmpMock) {
								return
							}
							dispatch('updateMock', tmpMock)
							if (historyOpen) {
								stepHistory?.deselect()
								stepHistoryPopover?.close()
							} else {
								selectJob(lastJob)
								preview = undefined
							}
						}}
					>
						Override pin
					</button>
				</span>
			{:else if infoMessage === 'restore'}
				<span>
					<button
						class="inline-block text-xs px-2 py-1 underline"
						on:click={() => {
							if (!tmpMock) {
								return
							}
							dispatch('updateMock', tmpMock)
							selectJob(undefined) // reset the job
							preview = undefined
							stepHistoryPopover?.close()
						}}
					>
						Restore pin <Pin size={14} class="inline" />
					</button>
				</span>
			{:else}
				<Button
					color="light"
					size="xs2"
					variant="contained"
					btnClasses={`bg-transparent ${
						mock?.enabled
							? 'text-white bg-blue-500 hover:text-primary hover:bg-blue-700 hover:text-gray-100'
							: ''
					}`}
					startIcon={{ icon: Pin }}
					iconOnly
					on:click={() => {
						if (mock?.enabled) {
							const newMock = {
								enabled: false,
								return_value: mock?.return_value
							}
							selectJob(undefined) // reset the job
							dispatch('updateMock', newMock)
						} else {
							let mockValue = jsonData

							if (selectedJob && 'result' in selectedJob) {
								mockValue = structuredClone(selectedJob.result)
								selectJob(undefined) // reset the job
								preview = undefined
							} else if (jsonData === 'never tested this far') {
								mockValue = { example: 'value' }
							}
							const newMock = {
								enabled: true,
								return_value: mockValue
							}
							dispatch('updateMock', newMock)
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
							jsonData = tmpMock?.return_value ?? undefined
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
							}}
							disabled={!mock?.enabled || !!connectingData}
						/>
						<svelte:fragment slot="text">
							{'Pin the output to allow editing'}
						</svelte:fragment>
					</Tooltip>
				{/if}
			{/if}
			{#if !isLoading && selectedJob && !preview && !mock?.enabled}
				<div class="w-grow min-w-0 flex gap-1 items-center">
					<OutputBadge job={selectedJob} class="grow min-w-16" />
					{#if selectedJob.id !== lastJob?.id}
						<button
							class="px-1 shrink-0 underline"
							on:click={() => {
								if (historyOpen) {
									stepHistory?.deselect()
								} else {
									selectJob(lastJob)
									preview = undefined
								}
							}}>See last result</button
						>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	{#if fullResult && !jsonView}
		{#if isLoading && (!mock?.enabled || preview === 'job')}
			<div class="p-2">
				<Loader2 class="animate-spin " />
			</div>
		{:else if (mock?.enabled || preview == 'mock') && preview != 'job'}
			<div class="break-words relative h-full p-2">
				<DisplayResult
					bind:forceJson
					workspaceId={undefined}
					jobId={undefined}
					result={mock?.return_value}
				/>
			</div>
		{:else if selectedJob != undefined && 'result' in selectedJob}
			<div class="break-words relative h-full p-2">
				{#if selectedJob.result != undefined}
					{#key selectedJob}
						<DisplayResult
							bind:forceJson
							workspaceId={selectedJob?.workspace_id}
							jobId={selectedJob?.id}
							result={selectedJob?.result}
						>
							<svelte:fragment slot="copilot-fix">
								<slot name="copilot-fix" />
							</svelte:fragment>
						</DisplayResult>
					{/key}
				{:else}
					null
				{/if}
			</div>
		{:else}
			<span class="px-1">Test to see the result here</span>
		{/if}
	{:else}
		<div class="grow min-h-0 p-2 rounded-sm w-full overflow-auto">
			{#if connectingData || simpleViewer}
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
						mock?.enabled && mock.return_value ? mock.return_value : jsonData,
						null,
						2
					)}
					class="h-full"
				/>
			{:else if mock?.enabled && preview != 'job'}
				<ObjectViewer
					json={moduleId
						? {
								[moduleId]: mock.return_value
						  }
						: mock.return_value}
					topBrackets={false}
					pureViewer={false}
				/>
			{:else if selectedJob != undefined && 'result' in selectedJob && selectedJob.result != undefined}
				<ObjectViewer
					json={moduleId
						? {
								[moduleId]: selectedJob.result
						  }
						: selectedJob.result}
					topBrackets={false}
					pureViewer={false}
				/>
			{:else if !lastJob}
				<div class="flex flex-col items-center justify-center h-full">
					<p class="text-xs text-secondary">
						Test this step to see results or <button
							class="text-blue-500 hover:text-blue-700 underline"
							on:click={() => {
								const newMock = {
									enabled: true,
									return_value: { example: 'value' }
								}
								dispatch('updateMock', newMock)
							}}>pin data<Pin size={16} class="inline" /></button
						>
					</p>
				</div>
			{:else}
				<ObjectViewer
					json={moduleId
						? {
								[moduleId]: jsonData
						  }
						: jsonData}
					topBrackets={false}
					pureViewer={false}
					{allowCopy}
				/>
			{/if}
		</div>
	{/if}
</div>
