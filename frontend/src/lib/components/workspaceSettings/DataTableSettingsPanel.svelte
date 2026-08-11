<script lang="ts">
	import { ExternalLink, Loader2 } from 'lucide-svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import CustomInstanceDbWizardModal from './CustomInstanceDbWizardModal.svelte'
	import Button from '../common/button/Button.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Password from '../Password.svelte'
	import Select from '../select/Select.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import CustomInstanceDbSelect from './CustomInstanceDbSelect.svelte'
	import DataTableConnectionReport from './DataTableConnectionReport.svelte'
	import SetupChecklist, { type SetupStep } from '../wizards/SetupChecklist.svelte'
	import Section from '../Section.svelte'
	import Label from '../Label.svelte'
	import {
		ResourceService,
		VariableService,
		WorkspaceService,
		type DataTableOrigin,
		type ListCustomInstanceDbsResponse,
		type TestDataTableConnectionResponse
	} from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import { useSupabaseOauth } from './supabaseOauth.svelte'
	import { isSupabaseHost } from './dataTableOrigin'
	import { derivePlan, newWizardState, runSetup, type WizardState } from './addDataTableModel'

	export type PanelDataTable = {
		name: string
		database: { resource_type: 'postgresql' | 'instance'; resource_path?: string }
		origin?: DataTableOrigin
		setup_incomplete?: boolean
	}

	type Props = {
		customInstanceDbs: ResourceReturn<ListCustomInstanceDbsResponse>
		confirmationModal: ConfirmationModalHandle
		existingNames: string[]
		/** Reloads the settings page's rows and health after anything here changes them. */
		onChanged: () => Promise<void>
	}

	let { customInstanceDbs, confirmationModal, existingNames, onChanged }: Props = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let dt: PanelDataTable | undefined = $state(undefined)

	let renameTo = $state('')
	let database = $state<{ resource_type: 'postgresql' | 'instance'; resource_path?: string }>({
		resource_type: 'postgresql'
	})
	let busy = $state(false)

	let check = $state<{
		loading: boolean
		report?: TestDataTableConnectionResponse
		error?: string
	}>({ loading: false })

	let newPassword = $state('')
	let resourceValue = $state<any>(undefined)
	let resourceEditor: ResourceEditorDrawer | undefined = $state(undefined)
	let openedInstanceDb: string | undefined = $state(undefined)

	/** Finish setup, for a data table whose wizard run never completed. */
	let resume = $state<{ steps: SetupStep[]; running: boolean } | undefined>(undefined)

	const supaOauth = useSupabaseOauth()

	/**
	 * `knownReport` is the settings page's own probe of this data table. Showing it straight
	 * away is what lets a row say "limited permissions" and land on the grants that fix it,
	 * rather than on a Test connection button that repeats work already done.
	 */
	export function open(target: PanelDataTable, knownReport?: TestDataTableConnectionResponse) {
		dt = target
		renameTo = target.name
		database = { ...target.database }
		check = { loading: false, report: knownReport }
		newPassword = ''
		resourceValue = undefined
		resume = undefined
		loadResource(target)
		drawer?.openDrawer()
	}

	// The row only knows the resource path; the panel is the one place that can afford to read
	// the resource itself, which is how a data table created before `origin` existed can still
	// be told apart from a plain Postgres one.
	async function loadResource(target: PanelDataTable) {
		if (target.database.resource_type !== 'postgresql' || !target.database.resource_path) return
		try {
			const resource = await ResourceService.getResource({
				workspace: $workspaceStore!,
				path: target.database.resource_path
			})
			resourceValue = resource.value
		} catch {
			resourceValue = undefined
		}
	}

	// All of these take the data table as a parameter: a `$derived` that reads state declared
	// in this same scope narrows `PanelDataTable | undefined` to `never` under the checker.
	function supabaseBacked(target: PanelDataTable | undefined, host: string | undefined): boolean {
		return target?.origin?.provider === 'supabase' || isSupabaseHost(host)
	}
	function renamed(target: PanelDataTable | undefined, to: string): boolean {
		return !!target && to.trim() !== target.name && !!to.trim()
	}
	function repointed(
		target: PanelDataTable | undefined,
		next: { resource_type: string; resource_path?: string }
	): boolean {
		return (
			!!target &&
			(next.resource_type !== target.database.resource_type ||
				next.resource_path !== target.database.resource_path)
		)
	}

	let isSupabase = $derived(supabaseBacked(dt, resourceValue?.host))
	let renameChanged = $derived(renamed(dt, renameTo))
	let renameTaken = $derived(
		renameChanged && existingNames.filter((n) => n !== dt?.name).includes(renameTo.trim())
	)
	let databaseChanged = $derived(repointed(dt, database))

	async function testConnection() {
		if (!dt) return
		check = { loading: true }
		try {
			const report = await WorkspaceService.testDataTableConnection({
				workspace: $workspaceStore!,
				datatableName: dt.name
			})
			check = { loading: false, report }
		} catch (err: any) {
			check = { loading: false, error: err?.body ?? err?.message ?? String(err) }
		}
	}

	/**
	 * Both a rename and a repoint go through the config form, which replaces the whole map, so
	 * the rest is read back and sent with it. Renames are declared separately because the
	 * backend cascades each data table's migration storage onto the new name.
	 */
	async function applyConfig(opts: { rename?: boolean; repoint?: boolean }) {
		if (!dt) return
		const confirmed = await confirmationModal.ask({
			title: opts.rename ? `Rename ${dt.name}?` : `Connect ${dt.name} to another database?`,
			children: opts.rename
				? `Every script that refers to datatable://${dt.name} will stop working until it is updated to datatable://${renameTo.trim()}.`
				: `${dt.name} will read and write a different database. Tables already created in the current one stay where they are.`,
			confirmationText: opts.rename ? 'Rename' : 'Connect'
		})
		if (!confirmed) return
		busy = true
		try {
			const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
			const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
			const current = datatables[dt.name]
			delete datatables[dt.name]
			const name = opts.rename ? renameTo.trim() : dt.name
			datatables[name] = {
				...current,
				database: opts.repoint ? { ...database } : current.database
			}
			await WorkspaceService.editDataTableConfig({
				workspace: $workspaceStore!,
				requestBody: {
					settings: { datatables },
					renames: opts.rename ? [{ from: dt.name, to: name }] : [],
					deleted_datatables: []
				}
			})
			sendUserToast(opts.rename ? `Renamed to ${name}` : `${name} now uses a different database`)
			drawer?.closeDrawer()
			await onChanged()
		} catch (err) {
			sendUserToast(String(err), true)
		} finally {
			busy = false
		}
	}

	/**
	 * Supabase never returns a database password through its API, so there is nothing to
	 * re-fetch and no point re-authorizing: the repair is to be told the current password, or
	 * a new one after it has been reset in Supabase.
	 */
	async function updatePassword() {
		if (!dt || !newPassword || !dt.database.resource_path) return
		busy = true
		try {
			const varPath = String(resourceValue?.password ?? '').startsWith('$var:')
				? String(resourceValue.password).slice('$var:'.length)
				: dt.database.resource_path
			await VariableService.updateVariable({
				workspace: $workspaceStore!,
				path: varPath,
				requestBody: { value: newPassword, is_secret: true }
			})
			newPassword = ''
			sendUserToast('Password updated')
			await testConnection()
		} catch (err) {
			sendUserToast(String(err), true)
		} finally {
			busy = false
		}
	}

	async function finishSetup() {
		if (!dt) return
		if (dt.origin?.provider === 'supabase' && !supaOauth.authed) {
			supaOauth.connect()
			return
		}
		const wiz: WizardState = newWizardState({
			name: dt.name,
			projectName: dt.origin?.project_name ?? '',
			folder: ''
		})
		wiz.review.name = dt.name
		if (dt.origin?.provider === 'supabase') {
			wiz.provider = 'supabase'
			wiz.supabase.mode = 'create'
			wiz.supabase.org = dt.origin.org
			wiz.supabase.region = dt.origin.region ?? wiz.supabase.region
			wiz.supabase.connectionMode = dt.origin.connection_mode === 'direct' ? 'direct' : 'session'
			const path = dt.database.resource_path ?? ''
			wiz.review.folder = path.split('/').slice(0, 2).join('/')
			wiz.review.resourceName = path.split('/').slice(2).join('/')
		} else if (dt.database.resource_type === 'instance') {
			wiz.provider = 'instance'
			wiz.instance = { mode: 'create', dbName: dt.database.resource_path }
		} else {
			wiz.provider = 'resource'
			wiz.own = { mode: 'pick', resourcePath: dt.database.resource_path, connectionString: '' }
		}

		resume = { steps: [], running: true }
		try {
			const resumeFrom = await derivePlan(wiz, {
				workspace: $workspaceStore!,
				supabaseToken: supaOauth.token
			})
			resume = { steps: resumeFrom, running: true }
			const result = await runSetup(wiz, {
				workspace: $workspaceStore!,
				username: $userStore?.username ?? 'admin',
				supabaseToken: supaOauth.token,
				confirmInstanceSetup: async () => true,
				onInstanceDbsChanged: async () => {
					await customInstanceDbs.refetch()
				},
				onProgress: (steps) => (resume = { steps, running: true }),
				onPoolerUnavailable: (reason) =>
					sendUserToast(`Connected directly instead of through the pooler: ${reason}`, true),
				resumeFrom
			})
			resume = { steps: resume?.steps ?? [], running: false }
			if (result.ok) {
				sendUserToast(`${dt.name} is ready`)
				drawer?.closeDrawer()
			}
			await onChanged()
		} catch (err) {
			sendUserToast(String(err), true)
			resume = { steps: resume?.steps ?? [], running: false }
		}
	}

	async function remove() {
		if (!dt) return
		const path = dt.database.resource_type === 'postgresql' ? dt.database.resource_path : undefined
		const mintedHere = !!dt.origin && dt.origin.provider !== 'resource'
		const confirmed = await confirmationModal.ask({
			title: `Delete ${dt.name}?`,
			children: isSupabase
				? `${dt.name} is removed from this workspace and scripts referring to it will fail. The Supabase project ${dt.origin?.project_name ?? ''} keeps running — delete it in Supabase if you no longer want it.`
				: `${dt.name} is removed from this workspace and scripts referring to it will fail. The underlying database is not touched.`,
			confirmationText: 'Delete data table'
		})
		if (!confirmed) return
		busy = true
		try {
			const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
			const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
			delete datatables[dt.name]
			await WorkspaceService.editDataTableConfig({
				workspace: $workspaceStore!,
				requestBody: { settings: { datatables }, renames: [], deleted_datatables: [dt.name] }
			})
			if (mintedHere && path) {
				// Only what this wizard created is offered for removal; a resource the user picked
				// belongs to them and may well be used elsewhere.
				const alsoResource = await confirmationModal.ask({
					title: 'Remove the saved connection too?',
					children: `Windmill created the resource and secret variable at ${path} for this data table. Delete them as well?`,
					confirmationText: 'Delete them'
				})
				if (alsoResource) {
					await ResourceService.deleteResource({ workspace: $workspaceStore!, path }).catch(
						() => {}
					)
					await VariableService.deleteVariable({ workspace: $workspaceStore!, path }).catch(
						() => {}
					)
				}
			}
			sendUserToast(`${dt.name} deleted`)
			drawer?.closeDrawer()
			await onChanged()
		} catch (err) {
			sendUserToast(String(err), true)
		} finally {
			busy = false
		}
	}

	function supabaseProjectUrl(origin: DataTableOrigin | undefined): string | undefined {
		if (!origin?.project_ref) return undefined
		return `https://supabase.com/dashboard/project/${origin.project_ref}`
	}
</script>

<Drawer bind:this={drawer} size="600px">
	<DrawerContent title={dt?.name ?? 'Data table'} on:close={() => drawer?.closeDrawer()}>
		{#if dt}
			<div class="flex flex-col gap-6">
				{#if dt.setup_incomplete}
					<div class="flex flex-col gap-2">
						<Alert type="warning" size="xs" title="Setup never finished">
							{dt.name} is recorded but not usable yet. Finishing picks up where it stopped and will
							not create a second project.
						</Alert>
						{#if resume}
							<SetupChecklist steps={resume.steps} />
						{/if}
						<div>
							<Button size="xs" variant="accent" loading={resume?.running} onClick={finishSetup}>
								{dt.origin?.provider === 'supabase' && !supaOauth.authed
									? 'Sign in to Supabase'
									: 'Finish setup'}
							</Button>
						</div>
					</div>
				{/if}

				<Section label="Connection" small class="flex flex-col gap-2">
					<dl
						class="grid grid-cols-[9rem_1fr] gap-y-1 gap-x-3 text-xs border rounded-md p-3 border-border-light"
					>
						{#if dt.origin?.project_name}
							<dt class="text-secondary">Supabase project</dt>
							<dd class="text-emphasis flex items-center gap-1">
								{dt.origin.project_name}
								{#if supabaseProjectUrl(dt.origin)}
									<a
										href={supabaseProjectUrl(dt.origin)}
										target="_blank"
										rel="noreferrer"
										class="text-blue-500"><ExternalLink size={12} /></a
									>
								{/if}
							</dd>
						{/if}
						{#if dt.origin?.org}
							<dt class="text-secondary">Organization</dt>
							<dd class="text-emphasis">{dt.origin.org}</dd>
						{/if}
						{#if dt.origin?.region}
							<dt class="text-secondary">Region</dt>
							<dd class="text-emphasis">{dt.origin.region}</dd>
						{/if}
						{#if dt.origin?.connection_mode}
							<dt class="text-secondary">Connection</dt>
							<dd class="text-emphasis">
								{dt.origin.connection_mode === 'session' ? 'Session pooler' : 'Direct'}
							</dd>
						{/if}
						<dt class="text-secondary">
							{dt.database.resource_type === 'instance' ? 'Windmill database' : 'Resource'}
						</dt>
						<dd class="text-emphasis font-mono">
							{#if dt.database.resource_type === 'postgresql' && dt.database.resource_path}
								{@const resourcePath = dt.database.resource_path}
								<Button
									size="xs2"
									variant="subtle"
									wrapperClasses="min-w-0 w-fit"
									btnClasses="font-mono text-emphasis"
									title="Edit {resourcePath}"
									on:click={() => resourceEditor?.initEdit(resourcePath)}
								>
									<span class="break-all">{resourcePath}</span>
								</Button>
							{:else if dt.database.resource_type === 'instance' && dt.database.resource_path}
								{@const instanceDb = dt.database.resource_path}
								<Button
									size="xs2"
									variant="subtle"
									wrapperClasses="min-w-0 w-fit"
									btnClasses="font-mono text-emphasis"
									title="Instance database setup for {instanceDb}"
									on:click={() => (openedInstanceDb = instanceDb)}
								>
									<span class="break-all">{instanceDb}</span>
								</Button>
							{:else}
								{dt.database.resource_path ?? '—'}
							{/if}
						</dd>
						{#if resourceValue?.host}
							<dt class="text-secondary">Host</dt>
							<dd class="text-emphasis font-mono break-all">{resourceValue.host}</dd>
						{/if}
						{#if dt.origin?.connected_by}
							<dt class="text-secondary">Connected by</dt>
							<dd class="text-emphasis">
								{dt.origin.connected_by}{dt.origin.connected_at
									? ` · ${new Date(dt.origin.connected_at).toLocaleDateString()}`
									: ''}
							</dd>
						{/if}
					</dl>
					<div class="flex items-center gap-2">
						<Button
							size="xs"
							variant="default"
							loading={check.loading}
							disabled={dt.setup_incomplete}
							onClick={testConnection}
						>
							Test connection
						</Button>
					</div>
					<DataTableConnectionReport name={dt.name} report={check.report} error={check.error} />
				</Section>

				{#if isSupabase}
					<Label label="Database password" class="gap-2">
						<Password bind:password={() => newPassword, (v) => (newPassword = v ?? '')} />
						<p class="text-2xs text-secondary">
							Supabase never exposes a project's database password, so signing in again cannot
							recover it. Paste the current one, or
							{#if supabaseProjectUrl(dt.origin)}
								<a
									href="{supabaseProjectUrl(dt.origin)}/database/settings"
									target="_blank"
									rel="noreferrer"
									class="text-blue-500 hover:underline">set a new one in Supabase</a
								>
							{:else}
								set a new one in Supabase
							{/if}
							— every existing connection to that project stops working when you do.
						</p>
						<div>
							<Button
								size="xs"
								variant="default"
								disabled={!newPassword}
								loading={busy}
								onClick={updatePassword}
							>
								Update password
							</Button>
						</div>
					</Label>
				{/if}

				<Label label="Database" class="gap-2">
					<p class="text-2xs text-secondary">
						Scripts address this data table by name, as
						<span class="font-mono">datatable://{dt.name}</span>. The database below is where its
						tables actually live. Connecting to another one moves nothing across — the data table
						then shows whatever that database already contains.
					</p>
					<div class="flex gap-2">
						<Select
							items={[
								{ value: 'postgresql', label: 'PostgreSQL' },
								{
									value: 'instance',
									label: 'Instance',
									disabled: isCloudHosted(),
									subtitle: $isCustomInstanceDbEnabled
										? undefined
										: isCloudHosted()
											? 'Not available on cloud'
											: 'Superadmin only'
								}
							]}
							bind:value={
								() => database.resource_type,
								(resource_type) => (database = { resource_type, resource_path: undefined })
							}
							class="w-32"
						/>
						<div class="flex-1">
							{#if database.resource_type === 'instance'}
								<CustomInstanceDbSelect
									{confirmationModal}
									{customInstanceDbs}
									bind:value={database.resource_path}
									tag="datatable"
								/>
							{:else}
								<ResourcePicker bind:value={database.resource_path} resourceType="postgresql" />
							{/if}
						</div>
					</div>
					<div>
						<Button
							size="xs"
							variant="default"
							disabled={!databaseChanged || !database.resource_path}
							loading={busy}
							onClick={() => applyConfig({ repoint: true })}
						>
							Connect to another database
						</Button>
					</div>
				</Label>

				<Label label="Name" class="gap-2">
					<TextInput bind:value={renameTo} />
					<p class="text-2xs text-secondary">
						{#if renameTaken}
							<span class="text-red-500">A data table called {renameTo.trim()} already exists.</span
							>
						{:else}
							Scripts refer to this data table as
							<span class="font-mono">datatable://{dt.name}</span>. Renaming it breaks every one of
							them until they are updated.
						{/if}
					</p>
					<div>
						<Button
							size="xs"
							variant="default"
							disabled={!renameChanged || renameTaken || !renameTo.trim()}
							loading={busy}
							onClick={() => applyConfig({ rename: true })}
						>
							Rename
						</Button>
					</div>
				</Label>

				<Section label="Danger zone" small class="flex flex-col gap-2">
					<div>
						<Button size="xs" variant="default" destructive loading={busy} onClick={remove}>
							Delete data table
						</Button>
					</div>
				</Section>
			</div>
		{:else}
			<div class="flex items-center gap-2 text-xs text-secondary">
				<Loader2 size={16} class="animate-spin" /> Loading
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<!-- Editing the resource can change the host this panel reports, so read it back. -->
<ResourceEditorDrawer
	bind:this={resourceEditor}
	on:refresh={() => {
		if (dt) loadResource(dt)
	}}
/>

<!-- Opened from inside this drawer, so it has to portal past it. -->
<CustomInstanceDbWizardModal
	{customInstanceDbs}
	{confirmationModal}
	tag="datatable"
	target="body"
	bind:opened={
		() =>
			openedInstanceDb
				? { dbname: openedInstanceDb, status: customInstanceDbs.current?.[openedInstanceDb] }
				: undefined,
		(v) => !v && (openedInstanceDb = undefined)
	}
/>
