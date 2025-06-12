<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Section from '$lib/components/Section.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { SaveIcon, EyeIcon, EyeOffIcon } from 'lucide-svelte'
	import { untrack } from 'svelte'

	let operatorWorkspaceSettings = $state({
		runs: true,
		schedules: true,
		resources: true,
		variables: true,
		triggers: true,
		audit_logs: true,
		groups: true,
		folders: true,
		workers: true
	})

	let originalSettings = $state({ ...untrack(() => operatorWorkspaceSettings) })
	let isChanged = $state(false)
	let currentWorkspace: string | null = $state(null)

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
		runs: { title: 'Runs', description: 'View runs' },
		schedules: { title: 'Schedules', description: 'View schedules' },
		resources: { title: 'Resources', description: 'View resources' },
		variables: { title: 'Variables', description: 'View variables' },
		triggers: { title: 'Triggers', description: 'View all triggers (HTTP, Websocket, Kafka)' },
		audit_logs: { title: 'Audit Logs', description: 'View audit logs' },
		groups: { title: 'Groups', description: 'View groups and group members' },
		folders: { title: 'Folders', description: 'View folders' },
		workers: { title: 'Workers', description: 'View workers and worker groups' }
	}

	$effect(() => {
		if ($workspaceStore && $workspaceStore !== currentWorkspace) {
			;(async () => {
				currentWorkspace = $workspaceStore
				const settings = await WorkspaceService.getSettings({
					workspace: $workspaceStore
				})
				if (settings.operator_settings !== null) {
					operatorWorkspaceSettings = settings.operator_settings ?? operatorWorkspaceSettings
					originalSettings = { ...operatorWorkspaceSettings }
				}
			})()
		}
	})

	$effect(() => {
		isChanged = JSON.stringify(operatorWorkspaceSettings) !== JSON.stringify(originalSettings)
	})

	let enableAllState = $derived(
		(() => {
			const values = Object.values(operatorWorkspaceSettings)
			if (values.every((v) => v === true)) return 'true'
			if (values.every((v) => v === false)) return 'false'
			return undefined
		})()
	)

	function toggleAllSettings(event) {
		const newValue = event.detail === true
		Object.keys(operatorWorkspaceSettings).forEach((key) => {
			operatorWorkspaceSettings[key] = newValue
		})
		operatorWorkspaceSettings = { ...operatorWorkspaceSettings }
	}
</script>

<div class="mt-6">
	<Section
		label="Operator Settings"
		collapsable={true}
		tooltip="Configure the operator visibility settings for your workspace. Toggle the settings you want to enable."
	>
		<div class="flex flex-col gap-4 my-4">
			<div class="flex flex-col gap-1">
				<div class="text-tertiary text-xs">
					Configure the operator visibility settings for your workspace. Toggle the settings you
					want to enable.
				</div>
			</div>
		</div>

		<div class="flex justify-end mb-2">
			<div class="flex justify-end"></div>
		</div>

		<div class="flex flex-col">
			<DataTable tableFixed={true} size="xs">
				<Head>
					<tr>
						<Cell head first>Section</Cell>
						<Cell head>Description</Cell>
						<Cell head last>
							<ToggleButtonGroup bind:selected={enableAllState} on:selected={toggleAllSettings}>
								{#snippet children({ item })}
									<ToggleButton
										icon={EyeIcon}
										small={true}
										value={'true'}
										label="Enable All"
										{item}
									/>
									<ToggleButton
										icon={EyeOffIcon}
										small={true}
										value={'false'}
										label="Disable All"
										{item}
									/>
								{/snippet}
							</ToggleButtonGroup>
						</Cell>
					</tr>
				</Head>
				<tbody class="divide-y bg-surface">
					{#each Object.entries(descriptions) as [key, { title, description }]}
						<tr>
							<Cell first>{title}</Cell>
							<Cell>{description}</Cell>
							<Cell last class="pl-8">
								<ToggleButtonGroup
									selected={operatorWorkspaceSettings[key] ? 'on' : 'off'}
									on:selected={({ detail }) => (operatorWorkspaceSettings[key] = detail === 'on')}
								>
									{#snippet children({ item })}
										<ToggleButton icon={EyeIcon} small={true} value={'on'} label="On" {item} />
										<ToggleButton icon={EyeOffIcon} small={true} value={'off'} label="Off" {item} />
									{/snippet}
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
	</Section>
</div>
