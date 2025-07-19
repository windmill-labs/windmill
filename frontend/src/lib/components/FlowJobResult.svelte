<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { Drawer } from './common'
	import AllFlowLogs from './AllFlowLogs.svelte'
	import type { DurationStatus } from './graph'
	import type { Writable } from 'svelte/store'
	import { untrack } from 'svelte'

	interface Props {
		waitingForExecutor?: boolean
		result: any
		logs: string | undefined
		col?: boolean
		noBorder?: boolean
		loading: boolean
		filename?: string | undefined
		jobId?: string | undefined
		tag?: string | undefined
		workspaceId?: string | undefined
		refreshLog?: boolean
		durationStates: Writable<Record<string, DurationStatus>> | undefined
		downloadLogs?: boolean
		tagLabel?: string | undefined
	}

	let {
		waitingForExecutor = false,
		result,
		logs = $bindable(),
		col = false,
		noBorder = false,
		loading,
		filename = undefined,
		jobId = undefined,
		tag = undefined,
		workspaceId = undefined,
		refreshLog = false,
		durationStates,
		downloadLogs = true,
		tagLabel = undefined
	}: Props = $props()

	let lastJobId: string | undefined = $state(undefined)
	let drawer: Drawer | undefined = $state(undefined)

	let iteration = 0
	let logOffset = 0

	async function diffJobId() {
		if (jobId != lastJobId) {
			lastJobId = jobId
			logs = undefined
			logOffset = 0
			iteration = 0
			getLogs()
		}
	}

	async function getLogs() {
		iteration += 1
		if (jobId) {
			const getUpdate = await JobService.getJobUpdates({
				workspace: workspaceId ?? $workspaceStore!,
				id: jobId,
				running: loading ?? false,
				logOffset: logOffset == 0 ? (logs?.length ? logs?.length + 1 : 0) : logOffset
			})
			logs = (logs ?? '').concat(getUpdate.new_logs ?? '')
			logOffset = getUpdate.log_offset ?? 0
		}
		if (refreshLog) {
			setTimeout(
				() => {
					if (refreshLog) {
						getLogs()
					}
				},
				iteration < 10 ? 1000 : iteration < 20 ? 2000 : 5000
			)
		}
	}
	$effect(() => {
		jobId
		untrack(() => {
			jobId != lastJobId && diffJobId()
		})
	})
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Explore all steps' logs" on:close={drawer.closeDrawer}
		><AllFlowLogs states={durationStates} /></DrawerContent
	>
</Drawer>
<div
	class:border={!noBorder}
	class="grid {!col
		? 'grid-cols-2'
		: 'grid-rows-2 max-h-screen'} shadow border border-tertiary-inverse grow overflow-hidden"
>
	<div class="bg-surface {col ? '' : 'max-h-80'} p-1 overflow-auto relative">
		<span class="text-tertiary">Result</span>
		{#if result !== undefined}
			<DisplayResult {workspaceId} {jobId} {filename} {result} />
		{:else if loading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="text-gray-400">No result (result is undefined)</div>
		{/if}
	</div>
	<div class="overflow-auto {col ? '' : 'max-h-80'} relative">
		<div class="absolute z-40 text-xs top-0 left-1"
			><button class="" onclick={drawer.openDrawer}>explore all steps' logs</button></div
		>
		<LogViewer
			{tagLabel}
			download={downloadLogs}
			content={logs ?? ''}
			{jobId}
			isLoading={waitingForExecutor}
			{tag}
		/>
	</div>
</div>
