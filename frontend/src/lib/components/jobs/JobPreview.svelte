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
	import { forLater } from '$lib/utils'

	const POPUP_HEIGHT = 240 as const

	export let job: Job | undefined
	let hovered = false
	let timeout: NodeJS.Timeout | undefined
	let watchJob: (id: string) => Promise<void>
	let args = job?.args
	let result: any
	let loaded = false
	let wrapper: HTMLElement
	let popupOnTop = true

	$: open = $openStore === job?.id

	async function instantOpen() {
		if (!open) {
			hovered = true
			if (!job) {
				return
			}
			popupOnTop = wrapper.getBoundingClientRect().top > POPUP_HEIGHT
			openStore.set(job.id)
			if (!loaded) {
				await tick()
				watchJob && watchJob(job.id)
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
			class="absolute z-50 {popupOnTop ? 'bottom-[35px]' : 'top-[35px]'} -left-10 bg-white rounded
			border border-gray-300 shadow-xl flex justify-start items-start w-[600px] h-80
			overflow-hidden"
		>
			<div class="w-1/2 h-full overflow-auto px-2">
				<JobArgs {args} tableClass="!pt-0 !min-w-0 !block" />
			</div>
			<div class="w-1/2 h-full overflow-auto p-2">
				{#if job && 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
					<div class="text-sm font-semibold text-gray-600 mb-1">
						<div>Job is scheduled for</div>
						<div>{new Date(job?.['scheduled_for']).toLocaleString()}</div>
					</div>
				{/if}
				{#if job?.type === Job.type.COMPLETED_JOB}
					<DisplayResult {result} disableExpand />
				{:else if job && `running` in job ? job.running : false}
					<div class="text-sm font-semibold text-gray-600 mb-1"> Job is still running </div>
					<LogViewer content={job?.logs} isLoading />
				{/if}
			</div>
		</div>
	{/if}
</div>
