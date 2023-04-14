<script context="module" lang="ts">
	const openStore = writable('')
</script>

<script lang="ts">
	import { tick } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Job } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import { writable } from 'svelte/store'
	import LogViewer from '../LogViewer.svelte'

	export let job: Job | undefined
	let timeout: NodeJS.Timeout | undefined
	let watchJob: (id: string) => Promise<void>
	let args = job?.args
	let result: any
	let loaded = false

	$: open = $openStore === job?.id
	$: completed = job?.type === Job.type.COMPLETED_JOB
	$: logs = job?.logs || logs

	async function instantOpen() {
		if (!job) {
			return
		}
		openStore.set(job.id)
		if (!loaded) {
			await tick()
			watchJob && watchJob(job.id)
		}
	}

	function staggeredOpen() {
		timeout = setTimeout(
			async () => {
				timeout = undefined
				await instantOpen()
			},
			loaded ? 300 : 500
		)
	}

	function close() {
		if (timeout) {
			clearTimeout(timeout)
			timeout = undefined
		}
		if (open) {
			openStore.set('')
		}
	}

	function onDone(event: { detail: Job }) {
		job = event.detail
		result = job['result']
		loaded = true
	}
</script>

<svelte:window on:keydown={({ key }) => ['Escape', 'Esc'].includes(key) && close()} />
<TestJobLoader bind:job bind:watchJob on:done={onDone} />

<div
	on:mouseenter={staggeredOpen}
	on:mouseleave={close}
	on:focusin={instantOpen}
	on:focusout={close}
	class="relative"
>
	<slot {open} />
	{#if open}
		<div
			transition:fade={{ duration: 100 }}
			class="absolute z-50 bottom-[65px] left-4 bg-white rounded border border-gray-300 shadow-xl
			flex justify-start items-start w-[600px] max-w-full h-60 overflow-hidden duration-100"
		>
			<div class="w-1/2 h-full overflow-auto px-2">
				<JobArgs {args} tableClass="!pt-0 !min-w-0 !block" />
			</div>
			<div class="w-1/2 h-full overflow-auto p-2">
				{#if completed}
					<DisplayResult {result} disableExpand />
				{:else}
					<div class="text-sm font-semibold text-gray-600 mb-1"> Job is still running </div>
					<LogViewer content={logs} isLoading wrapperClass="!h-[calc(100%-24px)]" />
				{/if}
			</div>
		</div>
	{/if}
</div>
