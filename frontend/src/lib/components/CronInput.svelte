<script context="module">
	export const OFFSET = new Date().getTimezoneOffset()
</script>

<script lang="ts">
	import { ScheduleService } from '$lib/gen'
	import { displayDate, formatCron, sendUserToast } from '$lib/utils'
	import CollapseLink from './CollapseLink.svelte'

	export let validCRON = true

	let preview: string[] = []
	let cronError = ''
	export let schedule: string = '0 0 12 * *'
	export let disabled = false
	let limit = 3

	$: handleScheduleInput(schedule)

	async function handleScheduleInput(input: string): Promise<void> {
		try {
			preview = await ScheduleService.previewSchedule({
				requestBody: { schedule: formatCron(input), offset: OFFSET }
			})
			cronError = ''
			validCRON = true
		} catch (err) {
			if (err.status == 400 && err.body.includes('cron')) {
				cronError = `Invalid cron expression`
				validCRON = false
			} else {
				sendUserToast(`Cannot preview: ${err}`, true)
				validCRON = false
			}
		}
	}
</script>

<div class="text-red-600 text-2xs grow">{cronError}</div>
<div class="flex flex-row items-end max-w-5xl">
	<label class="text-xs min-w-max mr-2 self-center" for="cron-schedule">CRON expression</label>
	<input
		class="inline-block"
		type="text"
		id="cron-schedule"
		name="cron-schedule"
		bind:value={schedule}
		{disabled}
	/>
</div>
{#if !disabled}
	<div class="flex flex-row text-xs text-blue-500 gap-3 pl-28 mb-2">
		<button
			on:click={() => {
				schedule = '0 */15 * * *'
				cronError = ''
			}}>every 15 min</button
		>
		<button
			on:click={() => {
				schedule = '0 0 * * * *'
				cronError = ''
			}}>every hour</button
		>
		<button
			on:click={() => {
				schedule = '0 0 8 * * *'
				cronError = ''
			}}>once a day at 8AM</button
		>
	</div>
{/if}

<CollapseLink text="preview next runs" open={true}>
	{#if preview && preview.length > 0}
		<div class="text-sm text-gray-700 border p-2 rounded-md">
			<div class="flex flex-row justify-between">The next runs will be scheduled at:</div>
			<ul class="list-disc mx-12">
				{#each preview.slice(0, limit) as p}
					<li class="mx-2 text-gray-700 text-sm">{displayDate(p)}</li>
				{/each}
				<li class="text-sm mx-2">...</li>
				{#if limit != 10}
					<button class="underline text-gray-400" on:click={() => (limit = 10)}>Load more</button>
				{:else}
					<button class="underline text-gray-400" on:click={() => (limit = 3)}>Load less</button>
				{/if}
			</ul>
		</div>
	{/if}
</CollapseLink>
