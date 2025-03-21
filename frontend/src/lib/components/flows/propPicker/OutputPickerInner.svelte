<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { Pin, History, Pen, Check, X, Loader2 } from 'lucide-svelte'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import ToggleSimple from '$lib/components/meltComponents/ToggleSimple.svelte'
	import StepHistory from './StepHistory.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { createEventDispatcher } from 'svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import type { Job } from '$lib/gen'
	import StatusBadge from '$lib/components/flows/propPicker/StatusBadge.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'

	export let jsonData: any = undefined
	export let prefix: string = ''
	export let allowCopy: boolean = false
	export let isConnecting: boolean = false
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
	export let selectedJob: Job | undefined = undefined
	export let forceJson: boolean = false
	export let testIsLoading: boolean = false
	export let scriptProgress: boolean = false

	const dispatch = createEventDispatcher<{
		updateMock: { enabled: boolean; return_value?: unknown }
	}>()

	let jsonView = false
	let clientHeight: number = 0
	let savedJsonData: any = {}
	let tmpMock: { enabled: boolean; return_value?: unknown } | undefined = undefined
	let error = ''
	let stepHistory: StepHistory | undefined = undefined
	let stepHistoryPopover: Popover | undefined = undefined
	let previewJob = false

	export function selectJob(job: Job | undefined) {
		selectedJob = job
		if (!job || !('result' in job)) {
			jsonData = savedJsonData
			return
		}

		savedJsonData = jsonData
		jsonData = job.result
		if (mock?.enabled) {
			const newMock = {
				enabled: true,
				return_value: job.result ?? {}
			}
			tmpMock = newMock
			previewJob = true
		}
	}
</script>

<div class="w-full h-full flex flex-col p-1" bind:clientHeight>
	<div class="flex flex-row items-center justify-between w-full">
		<div class="flex flex-row items-center gap-0.5">
			<Popover
				bind:this={stepHistoryPopover}
				floatingConfig={{
					placement: 'left-start',
					offset: { mainAxis: 10, crossAxis: -6 },
					gutter: 0 // hack to make offset effective, see https://github.com/melt-ui/melt-ui/issues/528
				}}
				contentClasses="w-[275px]"
				{closeOnOutsideClick}
				disablePopup={isConnecting}
			>
				<svelte:fragment slot="trigger">
					<Button
						color="light"
						size="xs2"
						variant="contained"
						btnClasses="bg-transparent"
						startIcon={{ icon: History }}
						nonCaptureEvent
					/>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<div class="rounded-[inherit]" style={`height: ${clientHeight}px`}>
						<StepHistory
							bind:this={stepHistory}
							{moduleId}
							{getLogs}
							on:select={({ detail }) => {
								selectJob(detail)
							}}
						/>
					</div>
				</svelte:fragment>
			</Popover>
			{#if selectedJob && mock?.enabled && previewJob}
				<StatusBadge
					class="absolute"
					on:close={() => {
						stepHistory?.deselect()
						selectedJob = undefined
					}}
				>
					Update pinned data ?
					<svelte:fragment slot="action">
						<Button
							color="blue"
							size="xs2"
							startIcon={{ icon: Check }}
							on:click={() => {
								if (!tmpMock) {
									return
								}
								dispatch('updateMock', tmpMock)
								selectedJob = undefined
								previewJob = false
								stepHistoryPopover?.close()
							}}
						/>
					</svelte:fragment>
				</StatusBadge>
			{:else}
				<ToggleSimple
					disabled={isConnecting}
					pressed={mock?.enabled ?? false}
					on:pressedChange={({ detail }) => {
						if (mock?.enabled && !detail) {
							const newMock = {
								enabled: false,
								return_value: mock?.return_value
							}
							dispatch('updateMock', newMock)
						} else if (detail && !!mock) {
							let result = jsonData
							if (selectedJob && 'result' in selectedJob) {
								result = structuredClone(selectedJob.result)
								selectedJob = undefined
							}
							const newMock = {
								enabled: true,
								return_value: result ?? { example: 'value' }
							}
							dispatch('updateMock', newMock)
						}
					}}
				>
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
						nonCaptureEvent
					/>
				</ToggleSimple>

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
							disabled={!mock?.enabled || isConnecting}
						/>
						<svelte:fragment slot="text">
							{'Enable mock to edit the output'}
						</svelte:fragment>
					</Tooltip>
				{/if}
			{/if}
		</div>
	</div>

	{#if fullResult && !jsonView}
		{#if testIsLoading}
			{#if !scriptProgress}
				<div class="p-2">
					<Loader2 class="animate-spin m-auto" />
				</div>
			{/if}
		{:else if selectedJob != undefined && 'result' in selectedJob && selectedJob.result != undefined}
			<div class="break-words relative h-full p-2">
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
			</div>
		{:else if mock?.enabled}
			<div class="break-words relative h-full p-2">
				<DisplayResult
					bind:forceJson
					workspaceId={undefined}
					jobId={undefined}
					result={mock.return_value}
				/>
			</div>
		{:else}
			Test to see the result here
		{/if}
	{:else}
		<div class="grow min-h-0 p-2 rounded-sm w-full overflow-auto">
			{#if !jsonData || jsonData === 'never tested this far'}
				<div class="flex flex-col items-center justify-center h-full">
					<p class="text-xs text-secondary">Test this step to see results</p>
				</div>
			{:else if isConnecting}
				<ObjectViewer
					json={{
						[moduleId]: jsonData
					}}
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
			{:else if mock?.enabled && !previewJob}
				<ObjectViewer
					json={{
						[moduleId]: mock.return_value
					}}
					topBrackets={false}
					pureViewer={false}
				/>
			{:else if selectedJob != undefined && 'result' in selectedJob && selectedJob.result != undefined}
				<ObjectViewer
					json={{
						[moduleId]: selectedJob.result
					}}
					topBrackets={false}
					pureViewer={false}
				/>
			{:else}
				<ObjectViewer
					json={{
						[moduleId]: jsonData
					}}
					topBrackets={false}
					pureViewer={false}
					{allowCopy}
				/>
			{/if}
		</div>
	{/if}
</div>
