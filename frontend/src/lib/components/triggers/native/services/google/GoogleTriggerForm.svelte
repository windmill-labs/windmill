<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import GoogleDrivePicker from './GoogleDrivePicker.svelte'
	import GoogleCalendarPicker from './GoogleCalendarPicker.svelte'
	import { Eye, FileText } from 'lucide-svelte'
	import GoogleDriveIcon from '$lib/components/icons/GoogleDriveIcon.svelte'
	import GoogleCalendarIcon from '$lib/components/icons/GoogleCalendarIcon.svelte'

	interface Props {
		serviceConfig: Record<string, any>
		errors: Record<string, string>
		disabled?: boolean
		externalData?: any
		loading?: boolean
	}

	let {
		serviceConfig = $bindable(),
		errors = $bindable(),
		loading = $bindable(false),
		disabled = false
	}: Props = $props()

	let triggerType = $state<'drive' | 'calendar'>(serviceConfig.triggerType ?? 'drive')

	// Drive state
	let driveResourceId = $state<string>(serviceConfig.resourceId ?? '')
	let driveResourceName = $state<string>(serviceConfig.resourceName ?? '')
	let driveWatchMode = $state<'file' | 'all'>(
		serviceConfig.resourceId ? 'file' : serviceConfig.googleResourceId ? 'all' : 'file'
	)

	// Calendar state
	let calendarId = $state<string>(serviceConfig.calendarId ?? 'primary')
	let calendarName = $state<string>(serviceConfig.calendarName ?? '')

	// No async loading — signal ready immediately
	$effect(() => {
		loading = false
	})

	// Sync form state back to serviceConfig
	$effect(() => {
		if (triggerType === 'drive') {
			serviceConfig = {
				triggerType,
				watchMode: driveWatchMode,
				...(driveWatchMode === 'file'
					? { resourceId: driveResourceId, resourceName: driveResourceName }
					: {})
			}
		} else {
			serviceConfig = {
				triggerType,
				calendarId,
				calendarName
			}
		}
	})

	export function validate(): Record<string, string> {
		let serviceErrors: Record<string, string> = {}

		if (triggerType === 'drive') {
			if (driveWatchMode === 'file' && !driveResourceId?.trim()) {
				serviceErrors.resourceId = 'File ID is required'
			}
		} else if (triggerType === 'calendar') {
			if (!calendarId?.trim()) {
				serviceErrors.calendarId = 'Calendar ID is required'
			}
		}

		return serviceErrors
	}
</script>

<Section label="Google Trigger Configuration">
	<div class="flex flex-col gap-4">
		<div>
			<p class="block text-xs font-semibold text-primary mb-1">Trigger Type</p>
			<ToggleButtonGroup bind:selected={triggerType} {disabled}>
				{#snippet children({ item })}
					<ToggleButton value="drive" label="Drive" icon={GoogleDriveIcon} {item} />
					<ToggleButton value="calendar" label="Calendar" icon={GoogleCalendarIcon} {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>

		{#if triggerType === 'drive'}
			<div class="flex flex-col gap-2">
				<p class="block text-xs font-semibold text-primary">Watch Mode</p>
				<ToggleButtonGroup bind:selected={driveWatchMode} {disabled}>
					{#snippet children({ item })}
						<ToggleButton value="file" label="Specific file" icon={FileText} {item} />
						<ToggleButton value="all" label="All changes" icon={Eye} {item} />
					{/snippet}
				</ToggleButtonGroup>

				{#if driveWatchMode === 'file'}
					<p class="block text-xs font-semibold text-primary">File</p>
					<GoogleDrivePicker
						bind:resourceId={driveResourceId}
						bind:resourceName={driveResourceName}
						{disabled}
					/>
					{#if errors.resourceId}
						<p class="text-red-500 text-xs">{errors.resourceId}</p>
					{/if}
				{:else}
					<p class="text-tertiary text-xs">
						Watches all changes across your Google Drive. The trigger will fire whenever any file is
						created, modified, or deleted.
					</p>
				{/if}
			</div>
		{:else if triggerType === 'calendar'}
			<div class="flex flex-col gap-2">
				<p class="block text-xs font-semibold text-primary">Calendar</p>
				<GoogleCalendarPicker bind:calendarId bind:calendarName {disabled} />
				{#if errors.calendarId}
					<p class="text-red-500 text-xs">{errors.calendarId}</p>
				{/if}
				<div class="text-tertiary text-xs">
					<p>
						Requires the <a
							href="https://console.cloud.google.com/apis/library/calendar-json.googleapis.com"
							target="_blank"
							rel="noopener"
							class="underline">Google Calendar API</a
						> to be enabled in your Google Cloud project.
					</p>
				</div>
			</div>
		{/if}
	</div>
</Section>
