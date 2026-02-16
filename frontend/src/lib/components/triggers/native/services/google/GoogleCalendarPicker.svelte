<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { GoogleCalendarEntry } from '$lib/gen/types.gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Badge } from '$lib/components/common'
	import { Loader2, X, RefreshCw } from 'lucide-svelte'
	import GoogleCalendarIcon from '$lib/components/icons/GoogleCalendarIcon.svelte'

	interface Props {
		calendarId: string
		calendarName: string
		disabled?: boolean
	}

	let {
		calendarId = $bindable(),
		calendarName = $bindable(),
		disabled = false
	}: Props = $props()

	let calendars = $state<GoogleCalendarEntry[]>([])
	let loading = $state(false)

	async function loadCalendars() {
		if (!$workspaceStore) return

		loading = true
		try {
			calendars = await NativeTriggerService.listGoogleCalendars({
				workspace: $workspaceStore
			})
			if (calendarId && !calendarName) {
				const found = calendars.find((c) => c.id === calendarId)
				if (found) {
					calendarName = found.summary
				}
			}
		} catch (err: any) {
			sendUserToast(`Failed to load calendars: ${err.body || err.message}`, true)
			calendars = []
		} finally {
			loading = false
		}
	}

	function selectCalendar(cal: GoogleCalendarEntry) {
		calendarId = cal.id
		calendarName = cal.summary
	}

	function clearSelection() {
		calendarId = ''
		calendarName = ''
	}

	$effect(() => {
		if ($workspaceStore) {
			loadCalendars()
		}
	})
</script>

<div class="flex flex-col gap-2 border rounded-md p-2 bg-surface">
	<div class="flex items-center gap-1 text-2xs">
		<span class="font-medium text-secondary">Calendars</span>
		<div class="flex-1"></div>
		<button
			class="p-0.5 text-tertiary hover:text-secondary"
			title="Refresh"
			onclick={() => loadCalendars()}
			{disabled}
		>
			<RefreshCw size={12} />
		</button>
	</div>

	{#if calendarId}
		<div
			class="flex items-center gap-2 px-2 py-1 rounded bg-surface-selected text-secondary text-xs border"
		>
			<GoogleCalendarIcon width="14px" height="14px" />
			<span>
				Selected: <strong>{calendarName || calendarId}</strong>
			</span>
			<button
				class="ml-auto text-tertiary hover:text-secondary"
				onclick={clearSelection}
				{disabled}
			>
				<X size={14} />
			</button>
		</div>
	{/if}

	<div class="max-h-48 overflow-y-auto -mx-2 -mb-2 px-2 pb-1">
		{#if loading}
			<div class="flex items-center justify-center gap-2 py-4 text-xs text-tertiary">
				<Loader2 class="animate-spin" size={14} />
				Loading calendars...
			</div>
		{:else if calendars.length === 0}
			<div class="text-center py-4 text-xs text-tertiary">
				No calendars found. Check that the Google Calendar API is enabled.
			</div>
		{:else}
			{#each calendars as cal (cal.id)}
				<div
					class="flex items-center gap-2 px-2 py-1.5 text-xs hover:bg-surface-hover border-b border-light last:border-b-0
						{cal.id === calendarId ? 'bg-surface-selected' : ''}"
				>
					<GoogleCalendarIcon width="14px" height="14px" />
					<button
						class="flex-1 text-left truncate hover:underline"
						title="Select {cal.summary}"
						onclick={() => selectCalendar(cal)}
						{disabled}
					>
						{cal.summary}
					</button>
					{#if cal.primary}
						<Badge color="blue" small>Primary</Badge>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>
