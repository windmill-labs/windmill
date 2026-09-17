<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import TextInput from '../text_input/TextInput.svelte'
	import Password from '../Password.svelte'
	import Select from '../select/Select.svelte'
	import DataTable from '../table/DataTable.svelte'
	import Head from '../table/Head.svelte'
	import Row from '../table/Row.svelte'
	import Cell from '../table/Cell.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import {
		SettingService,
		type CustomInstanceDbTag,
		type ExternalInstancePgSetupReport
	} from '$lib/gen'
	import { instanceSettingsSaved } from '../instanceSettings'
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { resource } from 'runed'
	import { deepEqual } from 'fast-equals'
	import {
		CircleCheck,
		CircleX,
		KeyRound,
		Plus,
		Trash2,
		TriangleAlert,
		Wrench
	} from 'lucide-svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'external_instance_pg'
	const SSLMODES = ['verify-full', 'verify-ca', 'require', 'prefer', 'disable']

	let isDisabled = $derived(disabled || !$enterpriseLicense)

	function field(key: string): any {
		return $values[KEY]?.[key]
	}

	// Empty inputs are removed rather than sent: the backend reads every field as optional, and an
	// empty string would be a real (and invalid) host, port or sslmode.
	function setField(key: string, value: any) {
		const next = { ...($values[KEY] ?? {}) }
		if (value === '' || value === undefined || value === null) {
			delete next[key]
		} else {
			next[key] = key === 'port' ? Number(value) : value
		}
		$values[KEY] = Object.keys(next).length > 0 ? next : undefined
	}

	// Setup reads the saved settings, never the form, so it waits until the form is saved.
	const saved = resource(
		() => $instanceSettingsSaved,
		async () => {
			try {
				return (await SettingService.getGlobal({ key: KEY })) ?? undefined
			} catch {
				return undefined
			}
		}
	)
	let unsaved = $derived(!saved.loading && !deepEqual(saved.current ?? undefined, $values[KEY]))

	let refreshKey = $state(0)
	const status = resource(
		() => [$instanceSettingsSaved, refreshKey],
		async () => {
			try {
				return await SettingService.getExternalInstancePgStatus()
			} catch {
				return undefined
			}
		}
	)
	const databases = resource(
		() => [$instanceSettingsSaved, refreshKey],
		async () => {
			try {
				return await SettingService.listExternalInstancePgDatabases()
			} catch {
				return {}
			}
		}
	)

	let freshReport: ExternalInstancePgSetupReport | undefined = $state(undefined)
	let report = $derived(freshReport ?? status.current?.last_setup)
	let runningSetup: 'setup' | 'rotate' | undefined = $state(undefined)

	const confirmationModal = createAsyncConfirmationModal()

	async function runSetup(rotate: boolean) {
		if (rotate) {
			const ok = await confirmationModal.ask({
				title: 'Rotate passwords',
				children:
					'Windmill generates new passwords for its two roles on the external cluster. Jobs that connect afterwards use the new ones.',
				confirmationText: 'Rotate'
			})
			if (!ok) return
		}
		runningSetup = rotate ? 'rotate' : 'setup'
		try {
			freshReport = await SettingService.setupExternalInstancePg({
				requestBody: { rotate_passwords: rotate }
			})
			sendUserToast(
				freshReport.success ? 'External cluster is set up' : 'Setup failed, see the report below',
				!freshReport.success
			)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			runningSetup = undefined
			refreshKey++
		}
	}

	let newDbName = $state('')
	let newDbTag: CustomInstanceDbTag = $state('datatable')
	let creating = $state(false)

	async function createDatabase() {
		creating = true
		try {
			await SettingService.createExternalInstancePgDatabase({
				name: newDbName.trim(),
				requestBody: { tag: newDbTag }
			})
			sendUserToast(`Created database ${newDbName.trim()}`)
			newDbName = ''
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			creating = false
			refreshKey++
		}
	}

	async function dropDatabase(name: string) {
		const ok = await confirmationModal.ask({
			title: `Drop database ${name}`,
			children:
				'The database and everything in it is deleted from the external cluster. This cannot be undone.',
			confirmationText: 'Drop database'
		})
		if (!ok) return
		try {
			await SettingService.dropExternalInstancePgDatabase({ name })
			sendUserToast(`Dropped database ${name}`)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			refreshKey++
		}
	}

	let databaseEntries = $derived(Object.entries(databases.current ?? {}))
	let setUp = $derived(!!status.current?.last_setup?.success)
</script>

<div class="flex flex-col gap-6">
	{#if !$enterpriseLicense}
		<Alert
			type="info"
			title="External instance databases are an Enterprise Edition feature"
			size="xs"
		/>
	{/if}

	<div class="grid grid-cols-2 gap-x-2 gap-y-4">
		<div class="flex flex-col gap-1">
			<label for="external_pg_host" class="text-xs font-semibold text-emphasis">Host</label>
			<TextInput
				inputProps={{ id: 'external_pg_host', placeholder: 'db.example.com', disabled: isDisabled }}
				bind:value={() => field('host'), (v) => setField('host', v)}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="external_pg_port" class="text-xs font-semibold text-emphasis">Port</label>
			<TextInput
				inputProps={{
					id: 'external_pg_port',
					type: 'number',
					placeholder: '5432',
					disabled: isDisabled
				}}
				bind:value={() => field('port'), (v) => setField('port', v)}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="external_pg_user" class="text-xs font-semibold text-emphasis">Admin user</label>
			<TextInput
				inputProps={{ id: 'external_pg_user', placeholder: 'windmill_admin', disabled: isDisabled }}
				bind:value={() => field('user'), (v) => setField('user', v)}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="external_pg_password" class="text-xs font-semibold text-emphasis">Password</label>
			<Password
				id="external_pg_password"
				small
				disabled={isDisabled}
				bind:password={() => field('password'), (v) => setField('password', v)}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="external_pg_dbname" class="text-xs font-semibold text-emphasis">
				Maintenance database
			</label>
			<TextInput
				inputProps={{ id: 'external_pg_dbname', placeholder: 'postgres', disabled: isDisabled }}
				bind:value={() => field('dbname'), (v) => setField('dbname', v)}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="external_pg_sslmode" class="text-xs font-semibold text-emphasis">SSL mode</label>
			<Select
				id="external_pg_sslmode"
				items={SSLMODES.map((m) => ({ value: m, label: m }))}
				placeholder="verify-full (default)"
				clearable
				disabled={isDisabled}
				bind:value={() => field('sslmode'), (v) => setField('sslmode', v)}
			/>
		</div>
		<div class="col-span-2 flex flex-col gap-1">
			<label for="external_pg_root_cert" class="text-xs font-semibold text-emphasis">
				Root certificate (PEM)
			</label>
			<TextInput
				underlyingInputEl="textarea"
				inputProps={{
					id: 'external_pg_root_cert',
					placeholder: '-----BEGIN CERTIFICATE-----',
					rows: 3,
					disabled: isDisabled
				}}
				bind:value={() => field('root_certificate_pem'), (v) => setField('root_certificate_pem', v)}
			/>
			<span class="text-2xs text-secondary">
				Leave empty to verify against the system trust store.
			</span>
		</div>
	</div>

	<div class="flex flex-col gap-2">
		<div class="flex items-center gap-2">
			<Button
				unifiedSize="md"
				variant="accent"
				startIcon={{ icon: Wrench }}
				disabled={isDisabled || unsaved || !saved.current || !!runningSetup}
				loading={runningSetup === 'setup'}
				onclick={() => runSetup(false)}
			>
				Set up cluster
			</Button>
			<Button
				unifiedSize="md"
				variant="default"
				startIcon={{ icon: KeyRound }}
				disabled={isDisabled || unsaved || !saved.current || !setUp || !!runningSetup}
				loading={runningSetup === 'rotate'}
				onclick={() => runSetup(true)}
			>
				Rotate passwords
			</Button>
			{#if unsaved}
				<span class="text-xs text-secondary">Save the settings before setting the cluster up.</span>
			{:else if !saved.current}
				<span class="text-xs text-secondary">Fill in the connection and save to set it up.</span>
			{/if}
		</div>

		{#if report}
			<div class="flex flex-col gap-1 rounded-md border p-3 bg-surface-secondary">
				<div class="flex items-center justify-between text-xs">
					<span class="font-semibold text-emphasis">
						{report.success ? 'Last setup succeeded' : 'Last setup failed'}
					</span>
					<span class="text-secondary">{new Date(report.finished_at).toLocaleString()}</span>
				</div>
				<ul class="flex flex-col gap-1.5 mt-1">
					{#each report.steps as step, i (i)}
						<li class="flex gap-2 text-xs">
							{#if step.status === 'ok'}
								<CircleCheck size={14} class="text-green-600 dark:text-green-400 shrink-0 mt-0.5" />
							{:else if step.status === 'warning'}
								<TriangleAlert
									size={14}
									class="text-yellow-600 dark:text-yellow-400 shrink-0 mt-0.5"
								/>
							{:else}
								<CircleX size={14} class="text-red-600 dark:text-red-400 shrink-0 mt-0.5" />
							{/if}
							<span class="font-mono text-secondary shrink-0">{step.name}</span>
							<span class="text-primary break-words">{step.message}</span>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-2">
		<div class="flex flex-col gap-0.5">
			<span class="text-xs font-semibold text-emphasis">Databases</span>
			<span class="text-xs text-secondary">
				Databases Windmill created on this cluster. Workspaces use them by picking the
				<span class="font-semibold">External instance</span> type in their data table or Ducklake settings.
			</span>
		</div>
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Name</Cell>
					<Cell head>Used for</Cell>
					<Cell head>Used by</Cell>
					<Cell head last></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface-tertiary">
				{#if databaseEntries.length === 0}
					<Row>
						<Cell colspan={4} class="text-center text-xs text-secondary py-4">No database yet</Cell>
					</Row>
				{/if}
				{#each databaseEntries as [name, db] (name)}
					<Row>
						<Cell first class="font-mono text-xs">{name}</Cell>
						<Cell class="text-xs">{db.tag === 'ducklake' ? 'Ducklake' : 'Data table'}</Cell>
						<Cell class="text-xs">
							{(db.used_by_workspaces ?? []).join(', ') || '—'}
						</Cell>
						<Cell last class="text-right">
							<Button
								unifiedSize="sm"
								variant="subtle"
								startIcon={{ icon: Trash2 }}
								iconOnly
								disabled={isDisabled || (db.used_by_workspaces ?? []).length > 0}
								title={(db.used_by_workspaces ?? []).length > 0
									? 'Still used by a workspace'
									: `Drop ${name}`}
								onclick={() => dropDatabase(name)}
							/>
						</Cell>
					</Row>
				{/each}
			</tbody>
		</DataTable>
		<div class="flex items-center gap-2">
			<TextInput
				class="flex-1"
				inputProps={{
					id: 'external_pg_new_db',
					placeholder: 'New database name',
					disabled: isDisabled || !setUp
				}}
				bind:value={newDbName}
			/>
			<Select
				id="external_pg_new_db_tag"
				class="w-36"
				items={[
					{ value: 'datatable', label: 'Data table' },
					{ value: 'ducklake', label: 'Ducklake' }
				]}
				disabled={isDisabled || !setUp}
				bind:value={newDbTag}
			/>
			<Button
				unifiedSize="md"
				variant="default"
				startIcon={{ icon: Plus }}
				disabled={isDisabled || !setUp || !newDbName.trim() || creating}
				loading={creating}
				onclick={createDatabase}
			>
				Create database
			</Button>
		</div>
		{#if !setUp}
			<span class="text-xs text-secondary">Set the cluster up before creating databases.</span>
		{/if}
	</div>
</div>

<ConfirmationModal {...confirmationModal.props} />
