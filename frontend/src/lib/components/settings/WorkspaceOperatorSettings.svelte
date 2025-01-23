<script lang="ts">
	import { Button } from '$lib/components/common'
	import { onMount } from 'svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import List from '$lib/components/common/layout/List.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { writable } from 'svelte/store'
	import { sendUserToast } from '$lib/toast'
	import { SaveIcon, EyeIcon, EyeOffIcon } from 'lucide-svelte'

	let operatorWorkspaceSettings = {
		runs: true,
		schedules: true,
		resources: true,
		variables: true,
        triggers: true,
		audit_logs: true,
		groups: true,
		folders: true,
		workers: true
	}
	
	let originalSettings = { ...operatorWorkspaceSettings }
	let isChanged = false

	async function saveSettings() {
		try {
			await WorkspaceService.updateOperatorSettings({
				workspace: $workspaceStore!,
				requestBody: operatorWorkspaceSettings
			})
			originalSettings = { ...operatorWorkspaceSettings }
			isChanged = false
			sendUserToast('Operator settings saved successfully!', false)
		} catch (error) {
			console.error('Error updating operator settings:', error)
			sendUserToast('Failed to save operator settings.', true)
		}
	}

	const descriptions = {
		runs: { title: 'Runs', description: 'Manage and execute runs' },
		schedules: { title: 'Schedules', description: 'Manage schedules' },
		resources: { title: 'Resources', description: 'Access resources' },
		variables: { title: 'Variables', description: 'Manage variables' },
		triggers: { title: 'Triggers', description: 'Manage all triggers (HTTP, Websocket, Kafka)' },
		audit_logs: { title: 'Audit Logs', description: 'View audit logs' },
		groups: { title: 'Groups', description: 'Manage groups' },
		folders: { title: 'Folders', description: 'Organize folders' },
		workers: { title: 'Workers', description: 'Manage workers' }
	}

	onMount(async () => {
		const settings = await WorkspaceService.getSettings({
			workspace: $workspaceStore!
		})
		if (settings.operator_settings !== null) {
			operatorWorkspaceSettings = settings.operator_settings
			originalSettings = { ...operatorWorkspaceSettings }
		}
	})

	$: isChanged = JSON.stringify(operatorWorkspaceSettings) !== JSON.stringify(originalSettings)

	$: enableAllState = (() => {
		const values = Object.values(operatorWorkspaceSettings)
		if (values.every((v) => v === true)) return true
		if (values.every((v) => v === false)) return false
		return null
	})()

	function toggleAllSettings(event) {
		const newValue = event.detail === true
		Object.keys(operatorWorkspaceSettings).forEach((key) => {
			operatorWorkspaceSettings[key] = newValue
		})
		operatorWorkspaceSettings = { ...operatorWorkspaceSettings }
	}
</script>

<div class="mt-6">
	<PageHeader
		title="Operator Settings"
		primary={true}
		tooltip="Manage operator visibility settings for your workspace."
		documentationLink="https://www.windmill.dev/docs/core_concepts/authentification#adding-users-to-a-workspace"
	/>

	<div class="flex flex-col gap-4 my-4">
		<div class="flex flex-col gap-1">
			<div class="text-tertiary text-xs">
				Configure the operator visibility settings for your workspace. Toggle the settings you want
				to enable.
			</div>
		</div>
	</div>

	<div class="flex justify-end mb-2">
		<div class="flex justify-end" />
	</div>

	<div class="flex flex-col">
		<DataTable tableFixed={true} size="xs">
			<Head>
				<tr>
					<Cell head first>Section</Cell>
					<Cell head>Description</Cell>
					<Cell head last>
						<ToggleButtonGroup bind:selected={enableAllState} on:selected={toggleAllSettings}>
							<ToggleButton icon={EyeIcon} small={true} value={true} label="Enable All" />
							<ToggleButton icon={EyeOffIcon} small={true} value={false} label="Disable All" />
						</ToggleButtonGroup>
					</Cell>
				</tr>
			</Head>
			<tbody class="bg-white divide-y divide-gray-200">
				{#each Object.entries(descriptions) as [key, { title, description }]}
						<tr>
							<Cell first>{title}</Cell>
							<Cell>{description}</Cell>
							<Cell last class="pl-8">
                                <ToggleButtonGroup bind:selected={operatorWorkspaceSettings[key]}>
                                    <ToggleButton icon={EyeIcon} small={true} value={true} label="On" />
                                    <ToggleButton icon={EyeOffIcon} small={true} value={false} label="Off" />
                                </ToggleButtonGroup>
							</Cell>
						</tr>
				{/each}
			</tbody>
		</DataTable>
	</div>

	<div class="flex justify-end mt-4">
		<Button on:click={saveSettings} startIcon={{ icon: SaveIcon }} disabled={!isChanged}>
			Save
		</Button>
	</div>
</div>
