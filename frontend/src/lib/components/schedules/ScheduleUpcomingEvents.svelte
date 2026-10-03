<script lang="ts">
	import { ScheduleService } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'
	import { resource } from 'runed'

	interface Props {
		workspace: string
		path: string
		schedule: string
		timezone: string
		draftOnly?: boolean
	}

	let { workspace, path, schedule, timezone, draftOnly = false }: Props = $props()

	// The list endpoint omits cron_version, and the preview parses with the legacy v1 parser when it
	// is absent, so read it from the schedule. A deployed NULL is v1, as the executor parses it; a
	// draft-only row reads its draft, where an absent version is v2, the default for new schedules.
	// Mounted only while the tooltip is open, so each hover fetches fresh dates.
	const preview = resource(
		() => ({ workspace, path, schedule, timezone, draftOnly }),
		async ({ workspace, path, schedule, timezone, draftOnly }) => {
			const saved = await ScheduleService.getSchedule({ workspace, path, getDraft: draftOnly })
			const cronVersion = saved.cron_version ?? (draftOnly ? 'v2' : undefined)
			return ScheduleService.previewSchedule({
				requestBody: { schedule, timezone, cron_version: cronVersion }
			})
		}
	)

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
	{#if preview.error}
		<span class="text-secondary">Could not compute upcoming events</span>
	{:else if preview.current === undefined}
		<Loader2 size={14} class="animate-spin" />
	{:else}
		{#each preview.current as date (date)}
			<span>{format(new Date(date))}</span>
		{:else}
			<span class="text-secondary">No upcoming events</span>
		{/each}
	{/if}
</div>
