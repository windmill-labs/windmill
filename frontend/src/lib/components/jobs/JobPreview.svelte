<script context="module" lang="ts">
	export const openStore = writable('')
</script>

<script lang="ts">
	import { onDestroy, tick } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Job } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import { writable } from 'svelte/store'
	import LogViewer from '../LogViewer.svelte'

	import { Badge } from '../common'
	import { forLater } from '$lib/forLater'
	import DurationMs from '../DurationMs.svelte'

	const POPUP_HEIGHT = 320 as const

	export let id: string
	let job: Job | undefined = undefined
	let hovered = false
	let timeout: NodeJS.Timeout | undefined
	let watchJob: (id: string) => Promise<void>
	let result: any
	let loaded = false
	let wrapper: HTMLElement
	let popupOnTop = true

	$: open = $openStore === id

	async function instantOpen() {
		if (!open) {
			hovered = true
			popupOnTop = wrapper.getBoundingClientRect().top > POPUP_HEIGHT
			openStore.set(id)
			if (!loaded) {
				await tick()
				watchJob && watchJob(id)
			}
		} else {
			timeout && clearTimeout(timeout)
		}
	}

	function close() {
		hovered = false
		if (timeout) {
			clearTimeout(timeout)
			timeout = undefined
		}
		if (open) {
			openStore.set('')
		}
	}

	function staggeredClose() {
		hovered = false
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(
			async () => {
				timeout = undefined
				close()
			},
			loaded ? 100 : 300
		)
	}

	function onDone(event: { detail: Job }) {
		job = event.detail
		result = job['result']
		loaded = true
	}

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})
</script>

<svelte:window on:keydown={({ key }) => ['Escape', 'Esc'].includes(key) && close()} />
{#if hovered}
	<TestJobLoader bind:job bind:watchJob on:done={onDone} />
{/if}

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	on:mouseenter={instantOpen}
	on:mouseleave={staggeredClose}
	bind:this={wrapper}
	class="relative"
>
	<slot {open} />
	{#if open}
		<div
			transition:fade|local={{ duration: 50 }}
			class="absolute z-50 {popupOnTop ? 'bottom-[35px]' : 'top-[35px]'} -left-10 bg-surface rounded
			border shadow-xl flex justify-start items-start w-[600px] h-80
			overflow-hidden"
		>
			<div class="absolute bottom-0 right-1 flex justify-end gap-2 pb-0.5 z-50 bg-surface-primary">
				{#if job?.started_at}
					<Badge>{new Date(job?.['started_at']).toLocaleString()}</Badge>
				{/if}
				<Badge>
					Mem: {job?.['mem_peak'] ? `${(job['mem_peak'] / 1024).toPrecision(4)}MB` : 'N/A'}
				</Badge>
				{#if job?.['duration_ms']}
					<DurationMs duration_ms={job?.['duration_ms']} />
				{/if}
			</div>
			<div class="w-1/2 h-full overflow-auto">
				<JobArgs args={job?.args} />
			</div>
			<div class="w-1/2 h-full overflow-auto p-2">
				{#if job && 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
					<div class="text-sm font-semibold text-tertiary mb-1">
						<div>Job is scheduled for</div>
						<div>{new Date(job?.['scheduled_for']).toLocaleString()}</div>
					</div>
				{/if}
				{#if job?.type === Job.type.COMPLETED_JOB}
					<DisplayResult workspaceId={job?.workspace_id} jobId={job?.id} {result} disableExpand />
				{:else if job && `running` in job ? job.running : false}
					<div class="text-sm font-semibold text-tertiary mb-1"> Job is still running </div>
					<LogViewer
						jobId={job?.id}
						duration={job?.['duration_ms']}
						mem={job?.['mem_peak']}
						content={job?.logs}
						isLoading
						tag={job?.tag}
					/>
				{/if}
			</div>
		</div>
	{/if}
</div>
