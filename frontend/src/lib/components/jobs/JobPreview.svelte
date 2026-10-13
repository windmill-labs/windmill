<script module lang="ts">
	export const openStore = writable('')
</script>

<script lang="ts">
	import { onDestroy, tick } from 'svelte'
	import { fade } from 'svelte/transition'
	import { type Job } from '../../gen'
	import JobLoader from '../JobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'
	import { writable } from 'svelte/store'
	import LogViewer from '../LogViewer.svelte'

	import { Badge } from '../common'
	import { forLater } from '$lib/forLater'
	import DurationMs from '../DurationMs.svelte'
	import { twMerge } from 'tailwind-merge'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import Portal from '../Portal.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	const POPUP_HEIGHT = 320 as const

	interface Props {
		id: string
		children?: import('svelte').Snippet<[any]>
		class?: string
	}

	let { id, children, class: clazz }: Props = $props()

	let job: Job | undefined = $state(undefined)
	let hovered = $state(false)
	let timeout: number | undefined
	let result: any = $state()
	let jobLoader: JobLoader | undefined = $state()
	let loaded = false
	let wrapper: HTMLElement | undefined = $state()
	let popupOnTop = $state(true)
	// The popup is portaled to <body> at fixed coordinates so an ancestor that clips
	// overflow (a collapsible tree folder) can't cut it off. That makes it a sibling of the
	// trigger rather than a descendant: moving onto it leaves the wrapper, so it holds
	// itself open, and it closes on scroll since it no longer follows the row.
	let anchor: DOMRect | undefined = $state()
	let popupEl: HTMLElement | undefined = $state()

	let open = $derived($openStore === id)

	async function instantOpen() {
		if (!open) {
			hovered = true
			anchor = wrapper?.getBoundingClientRect()
			popupOnTop = (anchor?.top ?? 0) > POPUP_HEIGHT
			openStore.set(id)
			if (!loaded) {
				await tick()
				jobLoader?.watchJob(id, {
					done(job) {
						onDone(job)
					}
				})
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

	function onDone(njob: Job) {
		job = njob
		result = job['result']
		loaded = true
	}

	onDestroy(() => {
		timeout && clearTimeout(timeout)
		// `openStore` outlives this instance: left set, a remount of the same job would
		// open with no anchor to position against.
		if (open) openStore.set('')
	})
</script>

<svelte:window
	onkeydown={({ key }) => ['Escape', 'Esc'].includes(key) && close()}
	onscrollcapture={(e) => open && !popupEl?.contains(e.target as Node) && close()}
/>
<!-- `open` too: crossing from the trigger onto the portaled popup leaves the wrapper, and
     unmounting here would cancel a load still in flight. -->
{#if hovered || open}
	<JobLoader bind:job bind:this={jobLoader} />
{/if}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div onmouseenter={instantOpen} onmouseleave={staggeredClose} bind:this={wrapper} class="relative">
	{@render children?.({ open })}
	{#if open && anchor}
		<Portal>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				bind:this={popupEl}
				transition:fade|local={{ duration: 50 }}
				onmouseenter={instantOpen}
				onmouseleave={staggeredClose}
				class={twMerge(
					'fixed z-50 bg-surface rounded border shadow-md flex flex-col gap-4 items-start w-[600px] h-80 overflow-hidden',
					clazz
				)}
				style="left: {anchor.left - 40}px; {popupOnTop
					? `bottom: ${window.innerHeight - anchor.bottom + 35}px;`
					: `top: ${anchor.top + 35}px;`}"
			>
				<div class="w-full flex flex-row grow min-h-0 gap-2">
					<div class="w-1/2 h-full overflow-auto space-y-1">
						<span class="text-xs font-normal text-secondary">Arguments</span>
						<JobArgs
							id={job?.id}
							workspace={job?.workspace_id ?? $operatingWorkspace ?? 'no_w'}
							args={job?.args}
						/>
					</div>
					<div class="w-1/2 h-full overflow-auto space-y-1">
						{#if job && 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
							<div class="text-xs font-semibold text-emphasis mb-1">
								<div>Job is scheduled for</div>
								<div>{new Date(job?.['scheduled_for']).toLocaleString()}</div>
							</div>
						{/if}
						{#if job?.type === 'CompletedJob'}
							<span class="text-xs font-normal text-secondary mb-1">Result</span>
							<DisplayResult
								workspaceId={job?.workspace_id}
								jobId={job?.id}
								{result}
								disableExpand
								language={job?.language}
							/>
						{:else if job && `running` in job ? job.running : false}
							<div class="text-sm font-semibold text-primary mb-1"> Job is still running </div>
							<LogViewer
								jobId={job?.id}
								duration={job?.['duration_ms']}
								mem={job?.['mem_peak']}
								content={job?.logs}
								isLoading={job?.['running'] == false}
								tag={job?.tag}
							/>
						{/if}
					</div>
				</div>
				<div class="flex justify-end gap-2 pb-0.5 z-50 bg-surface-primary">
					{#if job?.started_at}
						<Badge>{new Date(job?.['started_at']).toLocaleString()}</Badge>
					{/if}
					<Badge>
						Mem: {job?.['mem_peak'] ? `${(job['mem_peak'] / 1024).toPrecision(4)}MB` : 'N/A'}
					</Badge>
					{#if job?.['duration_ms']}
						<DurationMs
							duration_ms={job?.['duration_ms']}
							self_wait_time_ms={job?.self_wait_time_ms}
							aggregate_wait_time_ms={job?.aggregate_wait_time_ms}
						/>
					{/if}
					{#if job?.['labels'] && Array.isArray(job?.['labels']) && job?.['labels'].length > 0}
						{#each job?.['labels'] as label}
							<Badge>Label: {label}</Badge>
						{/each}
					{/if}
				</div>
			</div>
		</Portal>
	{/if}
</div>
