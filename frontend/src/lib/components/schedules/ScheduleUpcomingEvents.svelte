<script lang="ts">
	import { ScheduleService } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'

	interface Props {
		workspace: string
		path: string
		schedule: string
		timezone: string
		draftOnly?: boolean
	}

	let { workspace, path, schedule, timezone, draftOnly = false }: Props = $props()

	// The list endpoint omits cron_version, and the preview parses with the legacy v1 parser when it
	// is absent, so read it from the deployed schedule. A draft-only row has none; new schedules are v2.
	async function loadPreview(
		workspace: string,
		path: string,
		schedule: string,
		timezone: string,
		draftOnly: boolean
	) {
		const cronVersion = draftOnly
			? 'v2'
			: (await ScheduleService.getSchedule({ workspace, path })).cron_version
		return ScheduleService.previewSchedule({
			requestBody: { schedule, timezone, cron_version: cronVersion ?? undefined }
		})
	}

	// Mounted only while the tooltip is open, so each hover fetches fresh dates.
	const preview = $derived(loadPreview(workspace, path, schedule, timezone, draftOnly))

	function formatter(timeZone: string) {
		const options: Intl.DateTimeFormatOptions = {
			weekday: 'short',
			day: '2-digit',
			month: 'short',
			year: 'numeric',
			hour: 'numeric',
			minute: 'numeric',
			second: 'numeric',
			timeZoneName: 'short'
		}
		try {
			return new Intl.DateTimeFormat('en-GB', { ...options, timeZone }).format
		} catch {
			return new Intl.DateTimeFormat('en-GB', { ...options, timeZone: 'UTC' }).format
		}
	}

	const format = $derived(formatter(timezone))
</script>

<div class="flex flex-col gap-1">
	<div class="font-semibold text-emphasis">Estimated upcoming events ({timezone})</div>
	{#await preview}
		<Loader2 size={14} class="animate-spin" />
	{:then dates}
		{#each dates as date (date)}
			<span>{format(new Date(date))}</span>
		{:else}
			<span class="text-secondary">No upcoming events</span>
		{/each}
	{:catch}
		<span class="text-secondary">Could not compute upcoming events</span>
	{/await}
</div>
