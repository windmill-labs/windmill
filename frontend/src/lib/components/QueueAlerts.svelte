<script lang="ts">
	import { onMount } from 'svelte'
	import { Button } from '$lib/components/common'
	import Section from '$lib/components/Section.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { Plus, Edit3, Save, X, Trash, ExternalLink } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { twMerge } from 'tailwind-merge'
	import { ConfigService, type Alert } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'
	import Badge from './common/badge/Badge.svelte'
	import { enterpriseLicense } from '$lib/stores'

	let queueAlertConfig = $state<Alert[]>([])
	let availableTags = $state<string[]>([])
	let configName = 'alert__job_queue_waiting'

	let editingRowIndex = $state<number>(-1)
	let editForm = $state<{
		tags_to_monitor: string[]
		jobs_num_threshold: string
		alert_cooldown_seconds: string
		alert_time_threshold_seconds: string
	}>({
		tags_to_monitor: [],
		jobs_num_threshold: '',
		alert_cooldown_seconds: '',
		alert_time_threshold_seconds: ''
	})

	let newAlertForm = $state({
		tags_to_monitor: [] as string[],
		jobs_num_threshold: '3',
		alert_cooldown_seconds: '600',
		alert_time_threshold_seconds: '30'
	})

	let addAlertOpen = $state(false)
	let formErrors = $state<Record<string, string>>({})
	let expandedTagRows = $state<number[]>([])

	const MAX_NUMBER_OF_TAGS_DISPLAYED = 10

	onMount(async () => {
		await fetchConfig()
		availableTags = await fetchWorkerTags()
	})

	async function fetchConfig() {
		try {
			const response = await ConfigService.getConfig({ name: configName })
			queueAlertConfig = response?.alerts || []
			expandedTagRows = []
		} catch (error) {
			console.error('Failed to fetch config:', error)
		}
	}

	async function fetchWorkerTags(): Promise<string[]> {
		try {
			const response = await ConfigService.listConfigs()
			const workerTagsSet = new Set<string>()

			response.forEach((config) => {
				if (config.name.startsWith('worker__') && Array.isArray(config.config?.worker_tags)) {
					config?.config?.worker_tags.forEach((tag) => workerTagsSet.add(tag))
				}
			})

			return Array.from(workerTagsSet)
		} catch (error) {
			console.error('Failed to fetch worker tags:', error)
			return []
		}
	}

	function startEditing(index: number) {
		editingRowIndex = index
		const config = queueAlertConfig[index]
		editForm = {
			tags_to_monitor: [...config.tags_to_monitor],
			jobs_num_threshold: config.jobs_num_threshold.toString(),
			alert_cooldown_seconds: config.alert_cooldown_seconds.toString(),
			alert_time_threshold_seconds: config.alert_time_threshold_seconds.toString()
		}
		// Reset expanded state when entering edit mode
		expandedTagRows = expandedTagRows.filter((i) => i !== index)
	}

	function cancelEdit() {
		editingRowIndex = -1
		formErrors = {}
	}

	async function saveEdit() {
		if (!validateForm(editForm)) return

		try {
			queueAlertConfig[editingRowIndex] = {
				name: 'Job Queue Alert',
				tags_to_monitor: editForm.tags_to_monitor,
				jobs_num_threshold: parseInt(editForm.jobs_num_threshold),
				alert_cooldown_seconds: parseInt(editForm.alert_cooldown_seconds),
				alert_time_threshold_seconds: parseInt(editForm.alert_time_threshold_seconds)
			}

			await saveQueueAlertConfig()
			editingRowIndex = -1
			formErrors = {}
			sendUserToast('Alert configuration updated successfully')
		} catch (error) {
			sendUserToast('Failed to update alert configuration', true)
		}
	}

	async function deleteAlert(index: number) {
		try {
			queueAlertConfig.splice(index, 1)
			await saveQueueAlertConfig()
			// Clean up expanded state for deleted row and shift indices down
			expandedTagRows = expandedTagRows
				.filter((i) => i !== index)
				.map((i) => (i > index ? i - 1 : i))
			sendUserToast('Alert deleted successfully')
		} catch (error) {
			sendUserToast('Failed to delete alert', true)
		}
	}

	function validateForm(form: typeof editForm | typeof newAlertForm): boolean {
		formErrors = {}
		let isValid = true

		if (form.tags_to_monitor.length === 0) {
			formErrors.tags_to_monitor = 'At least one tag is required'
			isValid = false
		}

		const jobsThreshold = parseInt(form.jobs_num_threshold)
		if (isNaN(jobsThreshold) || jobsThreshold < 1) {
			formErrors.jobs_num_threshold = 'Must be a positive number'
			isValid = false
		}

		const cooldown = parseInt(form.alert_cooldown_seconds)
		if (isNaN(cooldown) || cooldown < 1) {
			formErrors.alert_cooldown_seconds = 'Must be a positive number'
			isValid = false
		}

		const timeThreshold = parseInt(form.alert_time_threshold_seconds)
		if (isNaN(timeThreshold) || timeThreshold < 1) {
			formErrors.alert_time_threshold_seconds = 'Must be a positive number'
			isValid = false
		}

		return isValid
	}

	async function addNewAlert() {
		if (!validateForm(newAlertForm)) return

		try {
			queueAlertConfig.push({
				name: 'Job Queue Alert',
				tags_to_monitor: newAlertForm.tags_to_monitor,
				jobs_num_threshold: parseInt(newAlertForm.jobs_num_threshold),
				alert_cooldown_seconds: parseInt(newAlertForm.alert_cooldown_seconds),
				alert_time_threshold_seconds: parseInt(newAlertForm.alert_time_threshold_seconds)
			})

			await saveQueueAlertConfig()

			// Reset form
			newAlertForm = {
				tags_to_monitor: [],
				jobs_num_threshold: '3',
				alert_cooldown_seconds: '600',
				alert_time_threshold_seconds: '30'
			}

			addAlertOpen = false
			formErrors = {}
			sendUserToast('Alert added successfully')
		} catch (error) {
			sendUserToast('Failed to add alert', true)
		}
	}

	async function saveQueueAlertConfig() {
		await ConfigService.updateConfig({
			name: configName,
			requestBody: { alerts: queueAlertConfig }
		})
	}

	function safeSelectItems(items: string[]) {
		return items.map((item) => ({ label: item, value: item }))
	}
</script>

<Section
	label="Queue alerts"
	description={$enterpriseLicense
		? 'Configure alerts for queue monitoring based on worker tags and thresholds'
		: ''}
	eeOnly
>
	{#snippet action()}
		{#if $enterpriseLicense}
			<Popover
				bind:isOpen={addAlertOpen}
				closeButton
				placement="bottom-end"
				contentClasses="p-4 w-96 max-w-96"
			>
				{#snippet trigger()}
					<Button variant="default" unifiedSize="md" startIcon={{ icon: Plus }}
						>Add new alert</Button
					>
				{/snippet}
				{#snippet content()}
					<form class="flex flex-col gap-y-6">
						<h3 class="text-sm font-semibold text-emphasis">Add queue alert</h3>

						<div class="flex flex-col gap-y-1">
							<label for="new-tags" class="text-xs font-semibold text-emphasis">
								Worker tags to monitor
							</label>
							<span class="text-xs font-normal text-secondary">
								Tags that identify which workers to monitor for this alert
							</span>
							<div class="flex gap-2 items-start">
								<MultiSelect
									items={safeSelectItems(availableTags)}
									bind:value={newAlertForm.tags_to_monitor}
									createText="Press Enter to add custom tag"
									placeholder="Select or create tags..."
									error={!!formErrors.tags_to_monitor}
									class="flex-1"
									disablePortal
								/>
								{#if newAlertForm.tags_to_monitor.length === 0}
									<Button
										variant="default"
										unifiedSize="md"
										onclick={() => {
											newAlertForm.tags_to_monitor = [...availableTags]
										}}
									>
										Add all tags
									</Button>
								{/if}
							</div>
							{#if formErrors.tags_to_monitor}
								<span class="text-2xs font-normal text-red-500">{formErrors.tags_to_monitor}</span>
							{/if}
						</div>

						<div class="flex flex-col gap-y-1">
							<label for="new-jobs-threshold" class="text-xs font-semibold text-emphasis">
								Jobs count threshold
							</label>
							<span class="text-xs font-normal text-secondary">
								Trigger alert when queue exceeds this many jobs
							</span>
							<TextInput
								inputProps={{
									id: 'new-jobs-threshold',
									type: 'number',
									min: '1',
									placeholder: '3'
								}}
								bind:value={newAlertForm.jobs_num_threshold}
								size="sm"
								class="w-full"
								error={!!formErrors.jobs_num_threshold}
							/>
							{#if formErrors.jobs_num_threshold}
								<span class="text-2xs font-normal text-red-500"
									>{formErrors.jobs_num_threshold}</span
								>
							{/if}
						</div>

						<div class="flex flex-col gap-y-1">
							<label for="new-cooldown" class="text-xs font-semibold text-emphasis">
								Alert cooldown (seconds)
							</label>
							<span class="text-xs font-normal text-secondary">
								Wait time between alerts for the same condition
							</span>
							<TextInput
								inputProps={{
									id: 'new-cooldown',
									type: 'number',
									min: '1',
									placeholder: '600'
								}}
								bind:value={newAlertForm.alert_cooldown_seconds}
								size="sm"
								class="w-full"
								error={!!formErrors.alert_cooldown_seconds}
							/>
							{#if formErrors.alert_cooldown_seconds}
								<span class="text-2xs font-normal text-red-500"
									>{formErrors.alert_cooldown_seconds}</span
								>
							{/if}
						</div>

						<div class="flex flex-col gap-y-1">
							<label for="new-time-threshold" class="text-xs font-semibold text-emphasis">
								Time threshold (seconds)
							</label>
							<span class="text-xs font-normal text-secondary">
								How long the condition must persist before alerting
							</span>
							<TextInput
								inputProps={{
									id: 'new-time-threshold',
									type: 'number',
									min: '1',
									placeholder: '30'
								}}
								bind:value={newAlertForm.alert_time_threshold_seconds}
								size="sm"
								class="w-full"
								error={!!formErrors.alert_time_threshold_seconds}
							/>
							{#if formErrors.alert_time_threshold_seconds}
								<span class="text-2xs font-normal text-red-500"
									>{formErrors.alert_time_threshold_seconds}</span
								>
							{/if}
						</div>

						<div class="flex gap-x-2 pt-2 justify-end">
							<Button
								type="button"
								variant="default"
								unifiedSize="md"
								onclick={() => {
									addAlertOpen = false
									formErrors = {}
								}}
							>
								Cancel
							</Button>
							<Button
								onClick={addNewAlert}
								type="submit"
								variant="accent"
								unifiedSize="md"
								startIcon={{ icon: Plus }}>Add alert</Button
							>
						</div>
					</form>
				{/snippet}
			</Popover>
		{/if}
	{/snippet}

	{#if !$enterpriseLicense}
		<div class="text-xs text-primary">
			Queue Metric Alerts is an enterprise feature allowing you to monitor queues for waiting jobs.
			Please upgrade to access this functionality. <a
				href="https://www.windmill.dev/pricing"
				target="_blank"
				>Learn more about our plans <ExternalLink size={12} class="inline-block" /></a
			>
		</div>
	{:else if queueAlertConfig.length === 0}
		<div class="text-center py-8">
			<p class="text-sm text-secondary">No queue alerts configured</p>
			<p class="text-xs text-hint mt-1">Add your first alert to monitor queue conditions</p>
		</div>
	{:else}
		<div class="overflow-x-auto border rounded-md">
			<table class="w-full">
				<thead>
					<tr class="border-b bg-surface-secondary">
						<th class="text-left py-3 px-4 text-xs font-normal text-normal min-w-48">
							<span class="inline-flex items-center gap-1">
								Worker Tags
								<Tooltip>Tags that identify which workers to monitor for this alert</Tooltip>
							</span>
						</th>
						<th class="text-left py-3 px-4 text-xs font-normal text-normal">
							<span class="inline-flex items-center gap-1">
								Jobs Threshold
								<Tooltip>Trigger alert when queue exceeds this many jobs</Tooltip>
							</span>
						</th>
						<th class="text-left py-3 px-4 text-xs font-normal text-normal">
							<span class="inline-flex items-center gap-1">
								Cooldown (s)
								<Tooltip>Wait time between alerts for the same condition</Tooltip>
							</span>
						</th>
						<th class="text-left py-3 px-4 text-xs font-normal text-normal">
							<span class="inline-flex items-center gap-1">
								Time Threshold (s)
								<Tooltip>How long the condition must persist before alerting</Tooltip>
							</span>
						</th>
						<th class="text-right py-3 px-4 text-xs font-normal text-normal">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each queueAlertConfig as config, index}
						<tr
							class={twMerge(
								'text-xs text-primary',
								index !== queueAlertConfig.length - 1 ? 'border-b' : '',
								editingRowIndex === index ? 'bg-surface-selected' : ''
							)}
						>
							<td class="p-2">
								{#if editingRowIndex === index}
									<div class="flex gap-2 items-start">
										<MultiSelect
											items={safeSelectItems(availableTags)}
											bind:value={editForm.tags_to_monitor}
											onCreateItem={(tag) => {
												if (!editForm.tags_to_monitor.includes(tag)) {
													editForm.tags_to_monitor = [...editForm.tags_to_monitor, tag]
												}
											}}
											createText="Press Enter to add custom tag"
											placeholder="Select or create tags..."
											class="flex-1"
										/>
										{#if editForm.tags_to_monitor.length === 0}
											<Button
												variant="default"
												unifiedSize="md"
												onclick={() => {
													editForm.tags_to_monitor = [...availableTags]
												}}
											>
												Add all tags
											</Button>
										{/if}
									</div>
								{:else}
									{@const isExpanded = expandedTagRows.includes(index)}
									{@const tagsToShow =
										isExpanded || config.tags_to_monitor.length <= MAX_NUMBER_OF_TAGS_DISPLAYED
											? config.tags_to_monitor
											: config.tags_to_monitor.slice(0, MAX_NUMBER_OF_TAGS_DISPLAYED)}

									<div class="flex flex-wrap gap-1">
										{#each tagsToShow as tag}
											<Badge color="blue" small>
												{tag}
											</Badge>
										{/each}

										{#if config.tags_to_monitor.length > MAX_NUMBER_OF_TAGS_DISPLAYED && !isExpanded}
											<Badge
												clickable
												color="blue"
												small
												onclick={() => {
													expandedTagRows = [...expandedTagRows, index]
												}}
											>
												+ {config.tags_to_monitor.length}
											</Badge>
										{/if}
									</div>
								{/if}
							</td>
							<td class="p-2">
								{#if editingRowIndex === index}
									<TextInput
										inputProps={{
											type: 'number',
											min: '1'
										}}
										bind:value={editForm.jobs_num_threshold}
										size="sm"
										class="w-20"
									/>
								{:else}
									<span>{config.jobs_num_threshold}</span>
								{/if}
							</td>
							<td class="p-2">
								{#if editingRowIndex === index}
									<TextInput
										inputProps={{
											type: 'number',
											min: '1'
										}}
										bind:value={editForm.alert_cooldown_seconds}
										size="sm"
										class="w-24"
									/>
								{:else}
									<span>{config.alert_cooldown_seconds}</span>
								{/if}
							</td>
							<td class="p-2">
								{#if editingRowIndex === index}
									<TextInput
										inputProps={{
											type: 'number',
											min: '1'
										}}
										bind:value={editForm.alert_time_threshold_seconds}
										size="sm"
										class="w-24"
									/>
								{:else}
									<span>{config.alert_time_threshold_seconds}</span>
								{/if}
							</td>
							<td class="p-2">
								{#if editingRowIndex === index}
									<div class="flex items-center gap-2 justify-end">
										<Button
											variant="accent"
											unifiedSize="sm"
											startIcon={{ icon: Save }}
											onclick={saveEdit}
										>
											Save
										</Button>
										<Button
											variant="default"
											unifiedSize="sm"
											startIcon={{ icon: X }}
											onclick={cancelEdit}
										>
											Cancel
										</Button>
									</div>
								{:else}
									<div class="flex items-center gap-2 justify-end">
										<Button
											variant="default"
											unifiedSize="sm"
											startIcon={{ icon: Edit3 }}
											onclick={() => startEditing(index)}
											disabled={editingRowIndex !== -1}
										>
											Edit
										</Button>
										<Button
											variant="subtle"
											destructive
											unifiedSize="sm"
											onclick={() => deleteAlert(index)}
											disabled={editingRowIndex !== -1}
											startIcon={{ icon: Trash }}
										>
											Delete
										</Button>
									</div>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</Section>
