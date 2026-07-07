<script lang="ts">
	import { Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import Tab from '../common/tabs/Tab.svelte'
	import TabContent from '../common/tabs/TabContent.svelte'
	import Toggle from '../Toggle.svelte'
	import Select from '../select/Select.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { Shield, Plus, Trash2, RefreshCw, Loader2, TriangleAlert } from 'lucide-svelte'
	import {
		WorkspaceService,
		type DataTablePermissions,
		type DataTableGrant,
		type DataTablePolicy
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin, userStore } from '$lib/stores'

	let {
		workspace,
		datatable,
		disabled = false
	}: {
		workspace: string
		datatable: string
		disabled?: boolean
	} = $props()

	let open = $state(false)
	let loading = $state(false)
	let busy = $state(false)
	let log = $state<string | undefined>(undefined)
	let selectedTab = $state('access')

	// Advanced permissions are an enterprise feature; only workspace admins manage them.
	const ee = $derived(!!$enterpriseLicense)
	const canManage = $derived(!!$userStore?.is_admin || !!$superadmin)

	let enabled = $state(false)
	let grants = $state<DataTableGrant[]>([])
	let policies = $state<DataTablePolicy[]>([])

	const kindItems = [
		{ label: 'User', value: 'user' },
		{ label: 'Group', value: 'group' }
	]
	const accessItems = [
		{ label: 'No access', value: 'none' },
		{ label: 'Read', value: 'read' },
		{ label: 'Read & write', value: 'write' }
	]
	const commandItems = [
		{ label: 'All', value: 'all' },
		{ label: 'Select', value: 'select' },
		{ label: 'Insert', value: 'insert' },
		{ label: 'Update', value: 'update' },
		{ label: 'Delete', value: 'delete' }
	]

	async function load() {
		loading = true
		log = undefined
		try {
			const res = await WorkspaceService.getDatatablePermissions({
				workspace,
				datatableName: datatable
			})
			enabled = res?.enabled ?? false
			grants = res?.grants ?? []
			policies = res?.policies ?? []
		} catch (e: any) {
			sendUserToast(`Failed to load permissions: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			loading = false
		}
	}

	function openModal() {
		open = true
		load()
	}

	async function save() {
		busy = true
		try {
			const body: DataTablePermissions = { enabled, grants, policies }
			log = await WorkspaceService.setDatatablePermissions({
				workspace,
				datatableName: datatable,
				requestBody: body
			})
			sendUserToast('Permissions saved')
		} catch (e: any) {
			sendUserToast(`Failed to save permissions: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	async function sync() {
		busy = true
		try {
			log = await WorkspaceService.syncDatatablePermissions({
				workspace,
				datatableName: datatable
			})
			sendUserToast('Permissions re-synced')
		} catch (e: any) {
			sendUserToast(`Failed to sync permissions: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			busy = false
		}
	}

	function addGrant() {
		grants = [
			...grants,
			{ principal: { kind: 'user', name: '' }, table: undefined, access: 'read' }
		]
	}
	function removeGrant(i: number) {
		grants = grants.filter((_, j) => j !== i)
	}
	function addPolicy() {
		policies = [...policies, { table: '', name: '', command: 'all', principals: [] }]
	}
	function removePolicy(i: number) {
		policies = policies.filter((_, j) => j !== i)
	}
	function addPolicyPrincipal(p: DataTablePolicy) {
		p.principals = [...(p.principals ?? []), { kind: 'user', name: '' }]
	}
	function removePolicyPrincipal(p: DataTablePolicy, i: number) {
		p.principals = (p.principals ?? []).filter((_, j) => j !== i)
	}
</script>

<Button
	variant="default"
	size="sm"
	{disabled}
	startIcon={{ icon: Shield }}
	title="Manage data table permissions"
	on:click={openModal}
>
	Permissions
</Button>

<Modal2 bind:isOpen={open} title="Permissions — {datatable}" fixedWidth="lg" fixedHeight="lg">
	{#snippet headerLeft()}
		<Tooltip>
			Restrict who can read and write this data table's rows, enforced natively by Postgres roles
			and row-level security. Opt in below, then grant access per user/group and optionally add
			row-level policies. Workspace admins always keep full access.
		</Tooltip>
	{/snippet}

	<div class="flex flex-col gap-3 w-full grow min-h-0">
		{#if loading}
			<div class="flex items-center justify-center grow text-tertiary">
				<Loader2 size={18} class="animate-spin" />
			</div>
		{:else}
			<div class="flex items-center justify-between gap-2">
				<Toggle
					size="sm"
					checked={enabled}
					disabled={!ee || !canManage}
					eeOnly
					options={{
						right: 'Advanced permissions',
						rightTooltip:
							'When off, every workspace member has full access (legacy behavior). When on, access is restricted to the grants and policies below.'
					}}
					on:change={(e) => (enabled = e.detail)}
				/>
			</div>

			{#if !ee}
				<div class="flex items-center gap-2 text-xs text-yellow-600 dark:text-yellow-400">
					<TriangleAlert size={14} />
					Data table permissions require an enterprise license.
				</div>
			{/if}

			<Tabs bind:selected={selectedTab} class="grow min-h-0">
				<Tab value="access" label="Access" />
				<Tab value="policies" label="Row policies" />
				{#snippet content()}
					<TabContent value="access" class="grow min-h-0 overflow-auto pt-3">
						<div class="flex flex-col gap-2">
							<div class="text-xs text-tertiary">
								Grant a user or group access to a specific table, or to all tables (leave the table
								field empty).
							</div>
							{#each grants as grant, i (i)}
								<div class="flex items-center gap-2">
									<Select
										items={kindItems}
										bind:value={grant.principal.kind}
										disabled={!canManage}
										class="w-28"
									/>
									<TextInput
										bind:value={grant.principal.name}
										inputProps={{
											placeholder: grant.principal.kind === 'user' ? 'email' : 'group name',
											disabled: !canManage
										}}
									/>
									<TextInput
										bind:value={grant.table}
										inputProps={{ placeholder: 'all tables', disabled: !canManage }}
									/>
									<Select
										items={accessItems}
										bind:value={grant.access}
										disabled={!canManage}
										class="w-40"
									/>
									<Button
										variant="subtle"
										size="xs"
										iconOnly
										color="red"
										startIcon={{ icon: Trash2 }}
										disabled={!canManage}
										on:click={() => removeGrant(i)}
									/>
								</div>
							{/each}
							<div>
								<Button
									variant="subtle"
									size="xs"
									startIcon={{ icon: Plus }}
									disabled={!canManage}
									on:click={addGrant}
								>
									Add grant
								</Button>
							</div>
						</div>
					</TabContent>

					<TabContent value="policies" class="grow min-h-0 overflow-auto pt-3">
						<div class="flex flex-col gap-3">
							<div class="text-xs text-tertiary">
								Row-level policies filter which rows a principal can see or change. Use
								<code>wm_email()</code> for the current user's email, and standard SQL in the USING
								(read/delete) and WITH CHECK (insert/update) expressions, e.g.
								<code>owner = wm_email()</code>.
							</div>
							{#each policies as policy, i (i)}
								<div class="flex flex-col gap-2 border rounded-md p-3">
									<div class="flex items-center gap-2">
										<TextInput
											bind:value={policy.table}
											inputProps={{ placeholder: 'table', disabled: !canManage }}
										/>
										<TextInput
											bind:value={policy.name}
											inputProps={{ placeholder: 'policy name', disabled: !canManage }}
										/>
										<Select
											items={commandItems}
											bind:value={policy.command}
											disabled={!canManage}
											class="w-32"
										/>
										<Button
											variant="subtle"
											size="xs"
											iconOnly
											color="red"
											startIcon={{ icon: Trash2 }}
											disabled={!canManage}
											on:click={() => removePolicy(i)}
										/>
									</div>
									<div class="flex flex-col gap-1 pl-1">
										<span class="text-xs text-hint">Applies to</span>
										{#each policy.principals ?? [] as principal, j (j)}
											<div class="flex items-center gap-2">
												<Select
													items={kindItems}
													bind:value={principal.kind}
													disabled={!canManage}
													class="w-28"
												/>
												<TextInput
													bind:value={principal.name}
													inputProps={{
														placeholder: principal.kind === 'user' ? 'email' : 'group name',
														disabled: !canManage
													}}
												/>
												<Button
													variant="subtle"
													size="xs"
													iconOnly
													color="red"
													startIcon={{ icon: Trash2 }}
													disabled={!canManage}
													on:click={() => removePolicyPrincipal(policy, j)}
												/>
											</div>
										{/each}
										<div>
											<Button
												variant="subtle"
												size="xs"
												startIcon={{ icon: Plus }}
												disabled={!canManage}
												on:click={() => addPolicyPrincipal(policy)}
											>
												Add principal
											</Button>
										</div>
									</div>
									<TextInput
										bind:value={policy.using}
										inputProps={{
											placeholder: 'USING expression (e.g. owner = wm_email())',
											disabled: !canManage
										}}
									/>
									<TextInput
										bind:value={policy.check}
										inputProps={{
											placeholder: 'WITH CHECK expression (insert/update)',
											disabled: !canManage
										}}
									/>
								</div>
							{/each}
							<div>
								<Button
									variant="subtle"
									size="xs"
									startIcon={{ icon: Plus }}
									disabled={!canManage}
									on:click={addPolicy}
								>
									Add policy
								</Button>
							</div>
						</div>
					</TabContent>
				{/snippet}
			</Tabs>

			{#if log}
				<div
					class="text-xs font-mono whitespace-pre-wrap max-h-32 overflow-auto border rounded-md p-2 text-secondary"
				>
					{log}
				</div>
			{/if}

			<div class="flex justify-between gap-2 pt-2 border-t">
				<Button
					variant="subtle"
					size="sm"
					startIcon={{ icon: RefreshCw }}
					disabled={busy || !ee || !canManage || !enabled}
					title="Re-apply the stored config (e.g. after group changes or new tables)"
					on:click={sync}
				>
					Sync
				</Button>
				<Button variant="accent" size="sm" disabled={busy || !ee || !canManage} on:click={save}>
					Save
				</Button>
			</div>
		{/if}
	</div>
</Modal2>
