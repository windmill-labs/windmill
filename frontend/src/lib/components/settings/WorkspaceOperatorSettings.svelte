<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Section from '$lib/components/Section.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
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
		assets: true,
		triggers: true,
		audit_logs: true,
		groups: true,
		folders: true,
		workers: true
	})

	// Kept out of `operatorWorkspaceSettings` so the visibility table's "Enable all" never flips a
	// write right, and so its own row stays out of that table.
	let builder = $state(false)

	let originalSettings = $state({ ...untrack(() => operatorWorkspaceSettings), builder: false })
	let isChanged = $state(false)
	let currentWorkspace: string | null = $state(null)
	let confirmBuilderOpen = $state(false)

	const settingsPayload = $derived({ ...operatorWorkspaceSettings, builder })

	async function saveSettings() {
		try {
			await WorkspaceService.updateOperatorSettings({
				workspace: $workspaceStore!,
				requestBody: settingsPayload
			})
			originalSettings = { ...settingsPayload }
			isChanged = false
			sendUserToast('Operator settings saved successfully!', false)
		} catch (error) {
			console.error('Error updating operator settings:', error)
			sendUserToast('Failed to save operator settings.', true)
		}
	}

	function onSaveClicked() {
		if (builder && !originalSettings.builder) {
			confirmBuilderOpen = true
		} else {
			saveSettings()
		}
	}

	const descriptions = {
		runs: { title: 'Runs', description: 'View runs' },
		schedules: { title: 'Schedules', description: 'View schedules' },
		resources: { title: 'Resources', description: 'View resources' },
		variables: { title: 'Variables', description: 'View variables' },
		assets: { title: 'Assets', description: 'View assets' },
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
					const { builder: remoteBuilder, ...remoteVisibility } = settings.operator_settings ?? {}
					operatorWorkspaceSettings = { ...operatorWorkspaceSettings, ...remoteVisibility }
					builder = remoteBuilder ?? false
					originalSettings = { ...operatorWorkspaceSettings, builder }
				}
			})()
		}
	})

	$effect(() => {
		isChanged = JSON.stringify(settingsPayload) !== JSON.stringify(originalSettings)
	})

	const allDisabled = $derived(
		Object.values(operatorWorkspaceSettings).every((value) => value === false)
	)
	const allEnabled = $derived(
		Object.values(operatorWorkspaceSettings).every((value) => value === true)
	)
</script>

<Section
	label="Operator settings"
	collapsable={true}
	tooltip="Configure the operator visibility settings for your workspace. Toggle the settings you want to enable."
	description="Configure the operator visibility settings for your workspace. Toggle the settings you want to enable."
>
	{#snippet action()}
		<Button
			on:click={onSaveClicked}
			startIcon={{ icon: SaveIcon }}
			disabled={!isChanged}
			variant="accent"
		>
			Save operator settings
		</Button>
	{/snippet}

	<div class="flex flex-col gap-y-1 mb-4">
		<span class="text-xs font-semibold text-emphasis">Builder rights</span>
		<span class="text-xs font-normal text-secondary">
			Let operators compose flows and raw apps out of scripts and flows that are already deployed.
			They still cannot write code. Each operator then consumes a full seat instead of half a seat.
		</span>
		<Toggle
			bind:checked={builder}
			options={{ right: 'Operators can build flows and raw apps' }}
			size="xs"
		/>
	</div>

	<DataTable tableFixed={true} size="xs">
		<Head>
			<tr>
				<Cell head first>Section</Cell>
				<Cell head>Description</Cell>
				<Cell head last>
					<ToggleButtonGroup
						bind:selected={
							() => (allDisabled ? 'false' : allEnabled ? 'true' : ''),
							(v) => {
								Object.keys(operatorWorkspaceSettings).forEach((key) => {
									if (v === 'true') operatorWorkspaceSettings[key] = true
									if (v === 'false') operatorWorkspaceSettings[key] = false
								})
							}
						}
					>
						{#snippet children({ item })}
							<ToggleButton icon={EyeIcon} small={true} value={'true'} label="Enable All" {item} />
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
</Section>

<ConfirmationModal
	open={confirmBuilderOpen}
	title="Give operators builder rights"
	confirmationText="Enable builder rights"
	onCanceled={() => (confirmBuilderOpen = false)}
	onConfirmed={async () => {
		confirmBuilderOpen = false
		await saveSettings()
	}}
>
	<div class="flex flex-col gap-2 text-sm">
		<span>This applies to every operator of this workspace, not to a chosen few.</span>
		<span>
			Each of them then consumes a full seat instead of half a seat, which changes what this
			instance is billed.
		</span>
		<span>
			They can create, edit and delete flows and raw apps wherever their folder permissions already
			let them write. Review those permissions before enabling.
		</span>
	</div>
</ConfirmationModal>
