<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Loader2 } from 'lucide-svelte'
	import GoogleDriveIcon from '$lib/components/icons/GoogleDriveIcon.svelte'
	import GoogleCalendarIcon from '$lib/components/icons/GoogleCalendarIcon.svelte'

	interface Props {
		serviceConfig: Record<string, any>
		errors: Record<string, string>
		disabled?: boolean
		externalData?: any
		loading: boolean
	}

	let {
		serviceConfig = $bindable(),
		errors = $bindable(),
		disabled = false,
		externalData = undefined,
		loading = $bindable()
	}: Props = $props()

	let triggerType = $state<'drive' | 'calendar'>(serviceConfig.triggerType ?? 'drive')

	// Args objects for SchemaForm (only schema-defined fields)
	let driveArgs = $state<Record<string, any>>({
		resourceId: serviceConfig.resourceId ?? '',
		isFolder: serviceConfig.isFolder ?? false,
		includeSubfolders: serviceConfig.includeSubfolders ?? false
	})

	let calendarArgs = $state<Record<string, any>>({
		calendarId: serviceConfig.calendarId ?? 'primary'
	})

	// Sync form state back to serviceConfig
	$effect(() => {
		const args = triggerType === 'drive' ? driveArgs : calendarArgs
		serviceConfig = { triggerType, ...args }
	})

	const driveSchema = {
		type: 'object',
		properties: {
			resourceId: {
				type: 'string',
				title: 'File/Folder ID',
				description: 'The Google Drive file or folder ID to watch for changes'
			},
			isFolder: {
				type: 'boolean',
				title: 'Watch Folder',
				description: 'Enable to watch a folder for changes (uses Drive Changes API)',
				default: false
			},
			includeSubfolders: {
				type: 'boolean',
				title: 'Include Subfolders',
				description: 'Watch for changes in subfolders (only applies when watching a folder)',
				default: false
			}
		},
		required: ['resourceId']
	}

	const calendarSchema = {
		type: 'object',
		properties: {
			calendarId: {
				type: 'string',
				title: 'Calendar ID',
				description:
					'The calendar ID to watch. Use "primary" for your main calendar, or a specific calendar ID.',
				default: 'primary'
			}
		},
		required: ['calendarId']
	}

	export function validate(): Record<string, string> {
		let serviceErrors: Record<string, string> = {}

		if (triggerType === 'drive') {
			if (!driveArgs.resourceId?.trim()) {
				serviceErrors.resourceId = 'File/Folder ID is required'
			}
		} else if (triggerType === 'calendar') {
			if (!calendarArgs.calendarId?.trim()) {
				serviceErrors.calendarId = 'Calendar ID is required'
			}
		}

		return serviceErrors
	}

	// Apply external data when editing existing trigger
	let externalDataApplied = $state(false)

	$effect(() => {
		if (externalData && !externalDataApplied) {
			if (externalData.triggerType) {
				triggerType = externalData.triggerType
			}
			driveArgs = {
				resourceId: externalData.resourceId ?? driveArgs.resourceId,
				isFolder: externalData.isFolder ?? driveArgs.isFolder,
				includeSubfolders: externalData.includeSubfolders ?? driveArgs.includeSubfolders
			}
			calendarArgs = {
				calendarId: externalData.calendarId ?? calendarArgs.calendarId
			}
			externalDataApplied = true
			loading = false
		}
	})

	// No async loading in this form, so mark as ready immediately when no external data
	$effect(() => {
		if (!externalData) {
			externalDataApplied = false
			loading = false
		}
	})
</script>

<Section label="Google Trigger Configuration">
	{#if loading}
		<div class="flex items-center gap-2 text-secondary text-xs">
			<Loader2 class="animate-spin" size={16} />
			Loading configuration...
		</div>
	{:else}
		<div class="flex flex-col gap-4">
			<div>
				<p class="block text-xs font-semibold text-primary mb-1">Trigger Type</p>
				<ToggleButtonGroup
					bind:selected={triggerType}
					{disabled}
				>
					{#snippet children({ item })}
						<ToggleButton value="drive" label="Drive" icon={GoogleDriveIcon} {item} />
						<ToggleButton value="calendar" label="Calendar" icon={GoogleCalendarIcon} {item} />
					{/snippet}
				</ToggleButtonGroup>
			</div>

			{#if triggerType === 'drive'}
				<SchemaForm
					schema={driveSchema}
					bind:args={driveArgs}
					isValid={true}
					compact={true}
					prettifyHeader={true}
					{disabled}
				/>
				<div class="text-tertiary text-xs">
					<p>
						To find a file/folder ID, open it in Google Drive and copy the ID from the URL.
						For example: <code>https://drive.google.com/file/d/<strong>FILE_ID</strong>/view</code>
					</p>
					<p class="mt-1">
						Requires the <a href="https://console.cloud.google.com/apis/library/drive.googleapis.com" target="_blank" rel="noopener" class="underline">Google Drive API</a> to be enabled in your Google Cloud project.
					</p>
				</div>
			{:else if triggerType === 'calendar'}
				<SchemaForm
					schema={calendarSchema}
					bind:args={calendarArgs}
					isValid={true}
					compact={true}
					prettifyHeader={true}
					{disabled}
				/>
				<div class="text-tertiary text-xs">
					<p>
						Use <code>primary</code> to watch your main calendar. For other calendars, find the calendar
						ID in Calendar Settings under "Integrate calendar".
					</p>
					<p class="mt-1">
						Requires the <a href="https://console.cloud.google.com/apis/library/calendar-json.googleapis.com" target="_blank" rel="noopener" class="underline">Google Calendar API</a> to be enabled in your Google Cloud project.
					</p>
				</div>
			{/if}
		</div>
	{/if}
</Section>
