<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Password from '../Password.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { DataTable, Cell, Row } from '../table'
	import Head from '../table/Head.svelte'
	import {
		SettingService,
		type ExternalInstancePgSetupReport,
		type CustomInstanceDbTag
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense } from '$lib/stores'
	import {
		CheckCircle2,
		CirclePlus,
		Database,
		RefreshCw,
		Trash2,
		TriangleAlert,
		XCircle
	} from 'lucide-svelte'
	import type { Writable } from 'svelte/store'
	import EEOnly from '../EEOnly.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'external_instance_pg'

	// The form is local, never a key seeded into `values`: the page snapshots those at load and
	// bulk-saves whatever it holds, so an untouched form would be sent as a change and refused by
	// the setting's validator — failing an admin's unrelated edit in the same save.
	let form = $state<Record<string, any>>({})
	let seededFrom: unknown = undefined
	$effect(() => {
		const stored = $values[KEY]
		if (stored !== seededFrom) {
			seededFrom = stored
			form = stored ? { ...$state.snapshot(stored) } : {}
		}
	})

	let settingUp = $state(false)
	let rotatePasswords = $state(false)
	let rotateModalOpen = $state(false)
	let dropping = $state<string | undefined>(undefined)
	let dropModalName = $state<string | undefined>(undefined)
	let creating = $state(false)
	let newDbName = $state('')
	let newDbTag = $state<CustomInstanceDbTag>('datatable')

	let status = $state<
		| { configured: boolean; database_count: number; last_setup?: ExternalInstancePgSetupReport }
		| undefined
	>(undefined)
	let databases = $state<
		Record<string, { tag?: string; success?: boolean; used_by_workspaces?: string[] }>
	>({})
	let loadError = $state<string | undefined>(undefined)

	async function refresh() {
		try {
			status = await SettingService.getExternalInstancePgStatus()
			databases = status.configured ? await SettingService.listExternalInstancePgDatabases() : {}
			loadError = undefined
		} catch (e) {
			loadError = e?.body ?? e?.message ?? String(e)
		}
	}

	$effect(() => {
		refresh()
	})

	// Setup connects to the cluster with the stored setting, so what is on screen has to be
	// written before it runs.
	async function saveAndSetup(rotate: boolean) {
		settingUp = true
		try {
			const value = { ...$state.snapshot(form), sslmode: form.sslmode ?? 'verify-full' }
			await SettingService.setGlobal({ key: KEY, requestBody: { value } })
			// The page keeps its own copy of every setting and bulk-saves it: leaving the one it read
			// at load in place would let a later save of an unrelated setting revert this one.
			seededFrom = value
			$values[KEY] = value
			const report = await SettingService.setupExternalInstancePg({
				requestBody: { rotate_passwords: rotate }
			})
			await refresh()
			sendUserToast(
				report.success ? 'External cluster set up' : 'Setup finished with errors',
				!report.success
			)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			settingUp = false
			rotatePasswords = false
		}
	}

	async function createDatabase() {
		const name = newDbName.trim()
		if (!name) return
		creating = true
		try {
			await SettingService.createExternalInstancePgDatabase({
				name,
				requestBody: { tag: newDbTag }
			})
			newDbName = ''
			await refresh()
			sendUserToast(`Database ${name} created on the external cluster`)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			creating = false
		}
	}

	async function dropDatabase(name: string) {
		dropping = name
		try {
			await SettingService.dropExternalInstancePgDatabase({ name })
			await refresh()
			sendUserToast(`Database ${name} dropped`)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			dropping = undefined
		}
	}

	let databaseEntries = $derived(Object.entries(databases))
</script>

<div class="flex flex-col gap-6">
		{#if !$enterpriseLicense}
			<EEOnly />
		{/if}

		<Alert type="info" title="A Postgres cluster Windmill administers" size="xs">
			Windmill creates the databases for data tables and Ducklake catalogs on this cluster, and
			manages the roles they connect as. The admin login below needs <span class="font-mono"
				>CREATEDB</span
			>
			and <span class="font-mono">CREATEROLE</span>, and is never handed to a job.
		</Alert>

		<div class="flex flex-col gap-3">
			<div class="flex flex-wrap gap-3 items-end">
				<label class="flex flex-col gap-1 grow min-w-48">
					<span class="text-secondary text-xs">Host</span>
					<TextInput
						inputProps={{ disabled, placeholder: 'db.internal', id: 'external-instance-pg-host' }}
						bind:value={form.host}
					/>
				</label>
				<label class="flex flex-col gap-1 w-28">
					<span class="text-secondary text-xs">Port</span>
					<TextInput
						inputProps={{
							disabled,
							type: 'number',
							placeholder: '5432',
							id: 'external-instance-pg-port'
						}}
						bind:value={form.port}
					/>
				</label>
				<label class="flex flex-col gap-1 w-48">
					<span class="text-secondary text-xs">Admin user</span>
					<TextInput
						inputProps={{ disabled, placeholder: 'postgres', id: 'external-instance-pg-user' }}
						bind:value={form.user}
					/>
				</label>
			</div>

			<div class="flex flex-wrap gap-3 items-end">
				<div class="flex flex-col gap-1 grow min-w-48">
					<span class="text-secondary text-xs">Admin password</span>
					<Password small bind:password={form.password} />
				</div>
				<label class="flex flex-col gap-1 w-48">
					<span class="text-secondary text-xs">Maintenance database</span>
					<TextInput
						inputProps={{ disabled, placeholder: 'postgres', id: 'external-instance-pg-dbname' }}
						bind:value={form.dbname}
					/>
				</label>
				<div class="flex flex-col gap-1 w-44">
					<span class="text-secondary text-xs">
						SSL mode
						<Tooltip>
							{#snippet text()}
								Anything below verify-ca sends the managed roles' passwords to whichever server
								answers. Windmill trusts the system roots plus the certificate below.
							{/snippet}
						</Tooltip>
					</span>
					<Select
						{disabled}
						items={[
							{ value: 'verify-full', label: 'verify-full' },
							{ value: 'verify-ca', label: 'verify-ca' },
							{ value: 'require', label: 'require' },
							{ value: 'prefer', label: 'prefer' },
							{ value: 'disable', label: 'disable' }
						]}
						bind:value={form.sslmode}
						id="external-instance-pg-sslmode"
					/>
				</div>
			</div>

			<label class="flex flex-col gap-1">
				<span class="text-secondary text-xs">Root certificate (PEM)</span>
				<textarea
					{disabled}
					rows="3"
					class="text-xs font-mono"
					placeholder="-----BEGIN CERTIFICATE-----"
					id="external-instance-pg-root-cert"
					bind:value={form.root_certificate_pem}
				></textarea>
			</label>
		</div>

		<div class="flex flex-wrap gap-3 items-center">
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={disabled || settingUp || !form.host || !form.user}
				startIcon={{ icon: Database }}
				onClick={() => saveAndSetup(false)}
			>
				{settingUp && !rotatePasswords ? 'Setting up...' : 'Save and run setup'}
			</Button>
			<Button
				variant="default"
				unifiedSize="sm"
				disabled={disabled || settingUp || !status?.configured}
				startIcon={{ icon: RefreshCw }}
				onClick={() => (rotateModalOpen = true)}
			>
				Rotate passwords
			</Button>
			{#if status}
				<div class="text-xs text-secondary flex items-center gap-2">
					{#if status.configured}
						<span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
						Configured · {status.database_count} database{status.database_count === 1 ? '' : 's'}
					{:else}
						<span class="w-1.5 h-1.5 rounded-full bg-surface-disabled"></span>
						Not configured yet
					{/if}
				</div>
			{/if}
		</div>

		{#if loadError}
			<Alert type="error" title="Could not read the cluster status" size="xs">{loadError}</Alert>
		{/if}

		{#if status?.last_setup}
			{@const report = status.last_setup}
			<div class="flex flex-col gap-2">
				<div class="flex items-center gap-2">
					<span class="text-sm font-semibold text-emphasis">Last setup</span>
					<span class="text-xs text-secondary">{new Date(report.finished_at).toLocaleString()}</span
					>
				</div>
				<div class="flex flex-col gap-1 border rounded-md divide-y">
					{#each report.steps as step}
						<div class="flex gap-2 items-start px-3 py-2">
							<div class="pt-0.5">
								{#if step.status === 'ok'}
									<CheckCircle2 size={14} class="text-green-600 dark:text-green-400" />
								{:else if step.status === 'warning'}
									<TriangleAlert size={14} class="text-yellow-600 dark:text-yellow-400" />
								{:else}
									<XCircle size={14} class="text-red-600 dark:text-red-400" />
								{/if}
							</div>
							<div class="flex flex-col min-w-0">
								<span class="text-xs font-semibold text-emphasis">{step.name}</span>
								<span class="text-xs text-secondary break-words">{step.message}</span>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if status?.configured}
			<div class="flex flex-col gap-2">
				<span class="text-sm font-semibold text-emphasis">Databases on the cluster</span>
				<span class="text-xs text-secondary">
					Only databases created here can back a data table or a Ducklake catalog, and one in use
					cannot be dropped.
				</span>
				<DataTable size="xs">
					<Head>
						<tr>
							<Cell head first>Name</Cell>
							<Cell head>Tag</Cell>
							<Cell head>Used by</Cell>
							<Cell head last></Cell>
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each databaseEntries as [name, db]}
							<Row>
								<Cell first class="font-mono">{name}</Cell>
								<Cell>{db.tag ?? '-'}</Cell>
								<Cell class="text-secondary">
									{db.used_by_workspaces?.length ? db.used_by_workspaces.join(', ') : '-'}
								</Cell>
								<Cell last>
									<div class="flex justify-end">
										<Button
											variant="default"
											unifiedSize="2xs"
											color="red"
											startIcon={{ icon: Trash2 }}
											iconOnly
											wrapperClasses="w-fit"
											title="Drop {name} on the cluster"
											disabled={disabled || dropping === name}
											onClick={() => (dropModalName = name)}
										/>
									</div>
								</Cell>
							</Row>
						{:else}
							<Row>
								<Cell first colspan={4} class="text-secondary text-xs">
									No database yet. Create one below, then pick it in a workspace's data table or
									Ducklake settings.
								</Cell>
							</Row>
						{/each}
					</tbody>
				</DataTable>
				<div class="flex flex-wrap gap-2 items-center">
					<TextInput
						inputProps={{
							disabled,
							placeholder: 'dt_analytics',
							id: 'external-instance-pg-new-db'
						}}
						bind:value={newDbName}
						class="w-56"
					/>
					<div class="w-40">
						<Select
							{disabled}
							items={[
								{ value: 'datatable', label: 'Data table' },
								{ value: 'ducklake', label: 'Ducklake' }
							]}
							bind:value={newDbTag}
							id="external-instance-pg-new-db-tag"
						/>
					</div>
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: CirclePlus }}
						disabled={disabled || creating || !newDbName.trim()}
						onClick={createDatabase}
					>
						Create database
					</Button>
				</div>
			</div>
		{/if}
	</div>

<ConfirmationModal
	open={rotateModalOpen}
	title="Rotate the managed passwords"
	confirmationText="Rotate"
	on:canceled={() => (rotateModalOpen = false)}
	on:confirmed={() => {
		rotateModalOpen = false
		rotatePasswords = true
		saveAndSetup(true)
	}}
>
	<span class="text-sm">
		New passwords are generated for the roles Windmill manages on the cluster. Jobs running against
		those databases while the rotation happens can fail and have to be retried.
	</span>
</ConfirmationModal>

<ConfirmationModal
	open={dropModalName !== undefined}
	title="Drop {dropModalName} on the external cluster"
	confirmationText="Drop"
	on:canceled={() => (dropModalName = undefined)}
	on:confirmed={() => {
		const name = dropModalName
		dropModalName = undefined
		if (name) dropDatabase(name)
	}}
>
	<span class="text-sm">
		The database and everything in it are deleted on the cluster. This cannot be undone.
	</span>
</ConfirmationModal>
