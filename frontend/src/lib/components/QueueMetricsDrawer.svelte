<script lang="ts">
	import { onMount } from 'svelte'
	import { Drawer, DrawerContent, Button } from './common'
	import QueueMetricsDrawerInner from './QueueMetricsDrawerInner.svelte'
	import { ConfigService } from '$lib/gen'
	import Section from './Section.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Pencil, Trash, Check, PlusCircle, SaveIcon } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'
	import { enterpriseLicense } from '$lib/stores'

	type Alert = {
		name: string
		tags_to_monitor: string[]
		jobs_num_threshold: number
		alert_cooldown_seconds: number
		alert_time_threshold_seconds: number
	}

	let drawer: Drawer
	export function openDrawer() {
		drawer?.openDrawer()
	}

	let alerts: Alert[] = []

	let configName = 'alert__job_queue_waiting'
	let originalAlerts: Alert[] = []
	let newTag = ''
	let editingIndex = -1
	let changesMade = false
	let removedAlerts: Alert[] = []
	let stagedNewAlert = false
	let workerTags: string[] = []
	let filteredTags: string[] = []

	$: removedAlerts

	onMount(async () => {
		await fetchConfig()
		workerTags = await fetchWorkerTags()
	})

	async function fetchConfig() {
		try {
			const response = (await ConfigService.getConfig({ name: configName })) as { alerts: Alert[] }
			alerts = response.alerts || []
			originalAlerts = JSON.parse(JSON.stringify(alerts))
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

	function startEditing(index) {
		editingIndex = index
		updateWorkerTags()
	}

	function saveAlert(index) {
		const newAlert = alerts[index]

		if (newAlert.tags_to_monitor.length === 0) {
			sendUserToast('Please add at least one tag before saving.', true)
			return
		}

		if (
			newAlert.jobs_num_threshold <= 0 ||
			newAlert.alert_cooldown_seconds <= 0 ||
			newAlert.alert_time_threshold_seconds <= 0
		) {
			sendUserToast('All numeric values must be strictly positive.', true)
			return
		}

		const alertExists = originalAlerts.some(
			(alert) =>
				originalAlerts.indexOf(alert) !== index &&
				JSON.stringify(alert.tags_to_monitor.sort()) ===
					JSON.stringify(newAlert.tags_to_monitor.sort())
		)

		if (alertExists) {
			sendUserToast('An identical alert already exists.', true)
			return
		}

		editingIndex = -1
		changesMade = JSON.stringify(alerts) !== JSON.stringify(originalAlerts)
		stagedNewAlert = false
	}

	function stageDeleteAlert(index) {
		const alert = alerts[index]
		removedAlerts = [...removedAlerts, alert]
		changesMade =
			removedAlerts.length > 0 || JSON.stringify(alerts) !== JSON.stringify(originalAlerts)
	}

	function filterTags(event: Event) {
		const input = (event.target as HTMLInputElement).value
		filteredTags = workerTags.filter((tag) => tag.toLowerCase().includes(input.toLowerCase()))
	}

	function addTag(alertIndex, tag) {
		if (workerTags.includes(tag) && !alerts[alertIndex].tags_to_monitor.includes(tag)) {
			alerts[alertIndex].tags_to_monitor = [...alerts[alertIndex].tags_to_monitor, tag]
		}
		newTag = ''
		filteredTags = []
	}

	function removeTag(alertIndex, tag) {
		alerts[alertIndex].tags_to_monitor = alerts[alertIndex].tags_to_monitor.filter((t) => t !== tag)
	}

	async function applyConfig() {
		if (stagedNewAlert) {
			sendUserToast('New alert staged. Please save or cancel the changes before applying.', true)
			return
		}

		try {
			await ConfigService.updateConfig({ name: configName, requestBody: { alerts } })
			sendUserToast('Configuration updated successfully')
			alerts = alerts.filter((alert) => !removedAlerts.includes(alert))
			originalAlerts = JSON.parse(JSON.stringify(alerts))
			removedAlerts = []
			changesMade = false
			stagedNewAlert = false
		} catch (error) {
			console.error('Failed to update config:', error)
		}
	}

	async function cancelChanges() {
		alerts = [...alerts, ...removedAlerts]
		alerts = JSON.parse(JSON.stringify(originalAlerts))
		removedAlerts = []
		editingIndex = -1
		changesMade = false
		stagedNewAlert = false
	}

	function addNewAlert() {
		// alert already being added
		if (stagedNewAlert) {
			return
		}

		const newAlert = {
			name: 'New Alert',
			tags_to_monitor: [],
			jobs_num_threshold: 3,
			alert_cooldown_seconds: 600,
			alert_time_threshold_seconds: 30
		}

		alerts = [...alerts, newAlert]
		editingIndex = alerts.length - 1
		stagedNewAlert = true
		updateWorkerTags()
	}

	async function updateWorkerTags() {
		workerTags = await fetchWorkerTags()
	}

	function addAllTags(alertIndex) {
		alerts[alertIndex].tags_to_monitor = [
			...new Set([...alerts[alertIndex].tags_to_monitor, ...workerTags])
		]
		alerts = alerts // Trigger reactivity
	}
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Queue Metrics" on:close={drawer.closeDrawer}>
		<Section
			label="Queue Alert Settings"
			collapsable={true}
			tooltip="Manage queue alerts for monitoring queue metrics. These alerts will trigger a critical alert notification if the queue metrics are met."
			eeOnly={true}
		>
			{#if $enterpriseLicense}
				{#if changesMade}
					<div class="text-red-600 text-xs whitespace-nowrap pb-2">Non applied changes</div>
					<div class="flex gap-2 pb-2">
						<Button color="blue" size="xs" on:click={applyConfig}>
							<SaveIcon size={16} /> Apply Config
						</Button>
						<Button color="light" size="xs" on:click={cancelChanges}>Cancel</Button>
					</div>
				{/if}

				{#if alerts.length > 0}
					<div>
						<form on:submit|preventDefault>
							<table class="w-full border-collapse mb-2 text-xs table-auto">
								<thead class="bg-gray-200 text-left text-xs">
									<tr>
										<th class="p-2 min-w-[120px]">
											Name
											<Tooltip
												markdownTooltip="The name of the alert. This will be used in the crticial alert notification and help identify the alert."
											/>
										</th>
										<th class="p-2 w-full">
											Queue Tags to Monitor
											<Tooltip markdownTooltip="Queue tags to monitor for this alert." />
										</th>
										<th class="p-2 min-w-[65px]">
											Jobs
											<Tooltip
												markdownTooltip="Number of jobs threshold. An alert will be triggerd if more than this number of jobs are in the queue for at least the time threshold."
											/>
										</th>
										<th class="p-2 min-w-[115px]">
											Cooldown (s)
											<Tooltip
												markdownTooltip="Cooldown period in seconds. The cooldown period is the time after an alert is triggered that no new alerts will be triggered."
											/>
										</th>
										<th class="p-2 min-w-[105px]">
											Time (s)
											<Tooltip
												markdownTooltip="Time threshold in seconds. An alert will be triggered if more than 'n jobs threshold' have been in the queue for at least this amount of time."
											/>
										</th>
										<th class="p-2 min-w-[100px]"> Actions </th>
									</tr>
								</thead>
								<tbody>
									{#each alerts as alert, index}
										<tr
											class:staged-for-deletion={removedAlerts.includes(alert)}
											class:non-interactable={removedAlerts.includes(alert)}
										>
											<td class="border p-2">
												{#if editingIndex === index}
													<input type="text" bind:value={alert.name} class="w-full p-1" />
												{:else}
													{alert.name}
												{/if}
											</td>
											<td class="border p-2">
												{#if editingIndex === index}
													<div class="flex flex-wrap gap-1 mb-2">
														{#each alert.tags_to_monitor as tag}
															<span class="inline-block bg-blue-100 rounded px-2 py-1 text-xs">
																{tag}
																<button
																	on:click={() => removeTag(index, tag)}
																	aria-label="Remove Tag"
																	class="ml-1 text-black text-xs">x</button
																>
															</span>
														{/each}
													</div>
													<div class="flex items-center">
														<input
															type="text"
															bind:value={newTag}
															placeholder="Add tag"
															on:input={(e) => filterTags(e)}
															class="p-1 flex-grow mr-1"
														/>
														<button on:click={() => addTag(index, newTag)} aria-label="Add Tag">
															<PlusCircle size={16} />
														</button>
													</div>
													<!-- Add the new "Add All Tags" button here -->
													<button
														on:click={() => addAllTags(index)}
														class="text-xs hover:bg-gray-200 rounded px-2 py-1 mt-1"
													>
														Add All Tags
													</button>
													{#if filteredTags.length > 0}
														<ul class="autocomplete-list">
															{#each filteredTags as tag}
																<li>
																	<button type="button" on:click={() => addTag(index, tag)}
																		>{tag}</button
																	>
																</li>
															{/each}
														</ul>
													{/if}
												{:else}
													<div class="flex flex-wrap gap-1">
														{#each alert.tags_to_monitor as tag}
															<span class="inline-block bg-blue-100 rounded px-2 py-1 text-xs"
																>{tag}</span
															>
														{/each}
													</div>
												{/if}
											</td>
											<td class="border p-2">
												{#if editingIndex === index}
													<input
														type="number"
														bind:value={alert.jobs_num_threshold}
														class="w-full p-1"
													/>
												{:else}
													{alert.jobs_num_threshold}
												{/if}
											</td>
											<td class="border p-2">
												{#if editingIndex === index}
													<input
														type="number"
														bind:value={alert.alert_cooldown_seconds}
														class="w-full p-1"
													/>
												{:else}
													{alert.alert_cooldown_seconds}
												{/if}
											</td>
											<td class="border p-2">
												{#if editingIndex === index}
													<input
														type="number"
														bind:value={alert.alert_time_threshold_seconds}
														class="w-full p-1"
													/>
												{:else}
													{alert.alert_time_threshold_seconds}
												{/if}
											</td>
											<td class="border p-2">
												<div class="flex gap-3 justify-center items-center">
													{#if editingIndex === index}
														<button on:click={() => saveAlert(index)} aria-label="Save">
															<Check size={16} />
														</button>
													{:else}
														<button on:click={() => startEditing(index)} aria-label="Edit">
															<Pencil size={16} />
														</button>
														<button on:click={() => stageDeleteAlert(index)} aria-label="Delete">
															<Trash size={16} />
														</button>
													{/if}
												</div>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</form>
					</div>
				{/if}

				<!-- Button to Add New Alert at the Bottom of the Table -->
				<div class="flex">
					<Button color="blue" size="xs" on:click={addNewAlert}>
						<PlusCircle size={16} />
						Add New Alert
					</Button>
				</div>
			{:else}
				<p class="text-gray-600 text-sm">
					Queue Metric Alerts are an enterprise feature allowing you to monitor queues for waiting jobs. Please upgrade to access this functionality. 
					<a href="https://www.windmill.dev/docs/misc/plans_details" target="_blank" class="text-blue-500 underline">Learn more about our plans.</a>
				</p>
			{/if}
		</Section>
		<div class="p-8">
			<QueueMetricsDrawerInner />
		</div>
	</DrawerContent>
</Drawer>

<style>
	.staged-for-deletion {
		background-color: #f8d7da; /* Light red background for staged deletion */
	}
	.non-interactable {
		pointer-events: none; /* Disable pointer events */
		opacity: 0.5; /* Reduce opacity */
	}
	.autocomplete-list {
		border: 1px solid #ccc;
		max-height: 150px;
		overflow-y: auto;
		background-color: white;
		position: absolute;
		z-index: 1000;
	}
	.autocomplete-list li {
		padding: 8px;
		cursor: pointer;
	}
	.autocomplete-list li:hover {
		background-color: #f0f0f0;
	}
</style>