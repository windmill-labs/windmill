<script lang="ts">
	import { tick } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { Job } from '../../gen'
	import TestJobLoader from '../TestJobLoader.svelte'
	import DisplayResult from '../DisplayResult.svelte'
	import JobArgs from '../JobArgs.svelte'

	export let job: Job
	let open = false
	let timeout: NodeJS.Timeout | undefined
	let watchJob: (id: string) => Promise<void>
	let result: any, args: Job['args']

	$: loaded = result && args

	function setValues(event: { detail: Job }) {
		result = event.detail['result']
		args = event.detail.args
	}

	function staggeredOpen() {
		timeout = setTimeout(
			async () => {
				open = true
				timeout = undefined
				if (!loaded) {
					await tick()
					watchJob && watchJob(job.id)
				}
			},
			loaded ? 300 : 500
		)
	}

	function close() {
		console.log('close')
		if (timeout) {
			clearTimeout(timeout)
			timeout = undefined
		}
		open = false
	}
</script>

<TestJobLoader {job} bind:watchJob on:done={setValues} />
<svelte:window on:keydown={({ key }) => ['Escape', 'Esc'].includes(key) && close()} />

<div on:mouseenter={staggeredOpen} on:mouseleave={close} class="relative">
	<slot {open} />
	{#if open && loaded}
		<div
			transition:fade={{ duration: 200 }}
			class="absolute z-50 bottom-[71px] left-0 bg-white rounded-lg rounded-b-none border-x border-t
			border-gray-400 flex justify-start items-start w-[600px] max-w-full h-60 overflow-hidden"
		>
			<div class="w-1/2 h-full overflow-auto px-2">
				<!-- <tr>
					<th>Argument</th>
					<th>Value</th>
				</tr>
				<tbody>
					{#if args && Object.keys(args).length > 0}
						{#each Object.entries(args) as [arg, value]}
							<tr>
								<td class="font-semibold">{arg}</td>
								<td><ArgInfo {value} /></td>
							</tr>
						{/each}
					{:else if args}
						<tr><div class="text-gray-600 pt-2 pl-1 text-sm">No arguments</div></tr>
					{/if}
				</tbody> -->
				<JobArgs {args} tableClass="!pt-0 !min-w-0 !block" />
			</div>
			<div class="w-1/2 h-full overflow-auto p-2">
				<DisplayResult {result} />
			</div>
		</div>
	{/if}
</div>
