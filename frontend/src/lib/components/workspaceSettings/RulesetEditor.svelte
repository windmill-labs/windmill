<script lang="ts">
	import { Section, Button, DrawerContent } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		GroupService,
		UserService,
		WorkspaceService,
		type ProtectionRuleKind,
		type ProtectionRuleset
	} from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clone } from '$lib/utils'
	import { DEV_WORKSPACE_LOCK_RULE_NAME } from '$lib/workspaceProtectionRules.svelte'
	import { untrack } from 'svelte'
	import { ArrowRight, Plus, Save, Trash2 } from 'lucide-svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import Select from '$lib/components/select/Select.svelte'

	interface Props {
		rule?: ProtectionRuleset
		existingNames?: string[]
		onUpdate?: () => void
		onClose: () => void
	}

	let { rule, existingNames = [], onUpdate, onClose }: Props = $props()

	// Create mode vs Edit mode
	const isCreateMode = $derived(!rule)
	// The dev-workspace feature creates, finds and removes its rule by name, so this one name is
	// fixed. Its restrictions and bypassers stay editable.
	const isNameLocked = $derived(rule?.name === DEV_WORKSPACE_LOCK_RULE_NAME)

	// Helper function to check if a rule is in the array
	const hasRule = (ruleKind: string) => rule?.rules?.includes(ruleKind as any) ?? false

	// Editable state
	let name = $state(untrack(() => rule)?.name ?? '')
	let disableDirectDeployment = $state(hasRule('DisableDirectDeployment'))
	let disableFork = $state(hasRule('DisableWorkspaceForking'))
	let restrictDeployToDeployers = $state(hasRule('RestrictDeployToDeployers'))
	let restrictAnonymousAppDeployment = $state(hasRule('RestrictAnonymousAppDeployment'))
	let restrictPublicRunSharing = $state(hasRule('RestrictPublicRunSharing'))
	let restrictGuestAppDeployment = $state(hasRule('RestrictGuestAppDeployment'))
	let requireIsolatedForks = $state(hasRule('RequireIsolatedForks'))
	let selectedGroups = $state<string[]>(
		untrack(() => rule)?.bypass_groups?.map((g) => g.replace('g/', '')) ?? []
	)
	let selectedUsers = $state<string[]>(
		untrack(() => rule)?.bypass_users?.map((u) => u.replace('u/', '')) ?? []
	)
	type RoleMapping = { from: string | undefined; to: string | undefined }[]
	const mappingRows = (m: Record<string, string> | undefined): RoleMapping =>
		Object.entries(m ?? {}).map(([from, to]) => ({ from, to }))
	let forkRoleMapping = $state<RoleMapping>(mappingRows(untrack(() => rule)?.fork_role_mapping))

	// Initial state for unsaved changes tracking
	let initialName = $state(untrack(() => rule)?.name ?? '')
	let initialDisableDirectDeployment = $state(hasRule('DisableDirectDeployment'))
	let initialDisableFork = $state(hasRule('DisableWorkspaceForking'))
	let initialRestrictDeployToDeployers = $state(hasRule('RestrictDeployToDeployers'))
	let initialRestrictAnonymousAppDeployment = $state(hasRule('RestrictAnonymousAppDeployment'))
	let initialRestrictPublicRunSharing = $state(hasRule('RestrictPublicRunSharing'))
	let initialRestrictGuestAppDeployment = $state(hasRule('RestrictGuestAppDeployment'))
	let initialRequireIsolatedForks = $state(hasRule('RequireIsolatedForks'))
	let initialSelectedGroups = $state<string[]>(
		untrack(() => rule)?.bypass_groups
			? untrack(() => rule)!.bypass_groups.map((g) => g.replace('g/', ''))
			: []
	)
	let initialForkRoleMapping = $state<RoleMapping>(
		mappingRows(untrack(() => rule)?.fork_role_mapping)
	)
	let initialSelectedUsers = $state<string[]>(
		untrack(() => rule)?.bypass_users
			? untrack(() => rule)!.bypass_users.map((u) => u.replace('u/', ''))
			: []
	)

	// Available options
	let availableGroups = $state<string[]>([])
	let availableUsers = $state<string[]>([])
	let availableRoles = $state<{ value: string; label: string }[]>([])

	let saving = $state(false)

	// Load available groups and users
	async function loadAvailableGroups() {
		const groups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
		availableGroups = groups
	}

	async function loadAvailableUsers() {
		const users = await UserService.listUsernames({ workspace: $workspaceStore! })
		availableUsers = users
	}

	async function loadAvailableRoles() {
		const { roles } = await WorkspaceService.listWorkspaceRoles({ workspace: $workspaceStore! })
		availableRoles = roles.map((r) => ({ value: r.name, label: r.label }))
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadAvailableGroups()
				loadAvailableUsers()
				loadAvailableRoles()
			})
		}
	})

	// Computed properties
	const hasUnsavedChanges = $derived(
		isCreateMode
			? name.trim() !== '' ||
					disableDirectDeployment ||
					disableFork ||
					restrictDeployToDeployers ||
					restrictAnonymousAppDeployment ||
					restrictPublicRunSharing ||
					restrictGuestAppDeployment ||
					requireIsolatedForks ||
					selectedGroups.length > 0 ||
					selectedUsers.length > 0 ||
					forkRoleMapping.length > 0
			: name !== initialName ||
					disableDirectDeployment !== initialDisableDirectDeployment ||
					disableFork !== initialDisableFork ||
					restrictDeployToDeployers !== initialRestrictDeployToDeployers ||
					restrictAnonymousAppDeployment !== initialRestrictAnonymousAppDeployment ||
					restrictPublicRunSharing !== initialRestrictPublicRunSharing ||
					restrictGuestAppDeployment !== initialRestrictGuestAppDeployment ||
					requireIsolatedForks !== initialRequireIsolatedForks ||
					JSON.stringify([...selectedGroups].sort()) !==
						JSON.stringify([...initialSelectedGroups].sort()) ||
					JSON.stringify([...selectedUsers].sort()) !==
						JSON.stringify([...initialSelectedUsers].sort()) ||
					JSON.stringify(forkRoleMapping) !== JSON.stringify(initialForkRoleMapping)
	)

	const mappingError = $derived.by(() => {
		const seen = new Set<string>()
		for (const { from, to } of forkRoleMapping) {
			if (!from || !to) return 'Pick both roles of every mapping'
			if (from === to) return `${from} is mapped to itself`
			if (seen.has(from)) return `${from} is mapped twice`
			seen.add(from)
		}
		return undefined
	})

	const forkRoleMappingBody = $derived(
		Object.fromEntries(forkRoleMapping.map(({ from, to }) => [from!, to!]))
	)

	const nameError = $derived.by(() => {
		if (!name.trim()) return 'Name is required'
		if (isCreateMode) {
			if (existingNames.includes(name)) return 'Name already exists'
		} else {
			if (name !== initialName && existingNames.includes(name)) return 'Name already exists'
		}
		return undefined
	})

	const canSave = $derived(!nameError && !mappingError && hasUnsavedChanges)

	async function create() {
		if (!canSave || !$workspaceStore) return

		saving = true
		try {
			await WorkspaceService.createProtectionRule({
				workspace: $workspaceStore,
				requestBody: {
					name,
					rules: [
						...(disableDirectDeployment ? ['DisableDirectDeployment' as ProtectionRuleKind] : []),
						...(disableFork ? ['DisableWorkspaceForking' as ProtectionRuleKind] : []),
						...(restrictDeployToDeployers
							? ['RestrictDeployToDeployers' as ProtectionRuleKind]
							: []),
						...(restrictAnonymousAppDeployment
							? ['RestrictAnonymousAppDeployment' as ProtectionRuleKind]
							: []),
						...(restrictPublicRunSharing ? ['RestrictPublicRunSharing' as ProtectionRuleKind] : []),
						...(restrictGuestAppDeployment
							? ['RestrictGuestAppDeployment' as ProtectionRuleKind]
							: []),
						...(requireIsolatedForks ? ['RequireIsolatedForks' as ProtectionRuleKind] : [])
					],
					bypass_groups: selectedGroups,
					bypass_users: selectedUsers,
					fork_role_mapping: forkRoleMappingBody
				}
			})

			sendUserToast('Protection rule created successfully')
			onUpdate?.()
		} catch (error) {
			console.error('Failed to create protection rule:', error)
			sendUserToast('Failed to create protection rule', true)
		} finally {
			saving = false
		}
	}

	async function save() {
		if (!canSave || !$workspaceStore) return

		saving = true
		try {
			await WorkspaceService.updateProtectionRule({
				workspace: $workspaceStore,
				ruleName: initialName,
				requestBody: {
					name,
					rules: [
						...(disableDirectDeployment ? ['DisableDirectDeployment' as ProtectionRuleKind] : []),
						...(disableFork ? ['DisableWorkspaceForking' as ProtectionRuleKind] : []),
						...(restrictDeployToDeployers
							? ['RestrictDeployToDeployers' as ProtectionRuleKind]
							: []),
						...(restrictAnonymousAppDeployment
							? ['RestrictAnonymousAppDeployment' as ProtectionRuleKind]
							: []),
						...(restrictPublicRunSharing ? ['RestrictPublicRunSharing' as ProtectionRuleKind] : []),
						...(restrictGuestAppDeployment
							? ['RestrictGuestAppDeployment' as ProtectionRuleKind]
							: []),
						...(requireIsolatedForks ? ['RequireIsolatedForks' as ProtectionRuleKind] : [])
					],
					bypass_groups: selectedGroups,
					bypass_users: selectedUsers,
					fork_role_mapping: forkRoleMappingBody
				}
			})

			sendUserToast('Protection rule saved successfully')

			// Update initial state
			initialName = name
			initialDisableDirectDeployment = disableDirectDeployment
			initialDisableFork = disableFork
			initialRestrictDeployToDeployers = restrictDeployToDeployers
			initialRestrictAnonymousAppDeployment = restrictAnonymousAppDeployment
			initialRestrictPublicRunSharing = restrictPublicRunSharing
			initialRestrictGuestAppDeployment = restrictGuestAppDeployment
			initialRequireIsolatedForks = requireIsolatedForks
			initialSelectedGroups = clone(selectedGroups)
			initialSelectedUsers = clone(selectedUsers)
			initialForkRoleMapping = clone(forkRoleMapping)

			onUpdate?.()
		} catch (error) {
			console.error('Failed to save protection rule:', error)
			sendUserToast('Failed to save protection rule', true)
		} finally {
			saving = false
		}
	}

	// The required marker already says an empty name is missing; only a clash needs words.
	const shownNameError = $derived(name.trim() ? nameError : undefined)
</script>

<!-- `pl-11` lines the text up with the toggle's label: the `sm` track is 36px plus its 8px gap. -->
{#snippet description(text: string)}
	<p class="text-xs text-secondary pl-11">{text}</p>
{/snippet}

<DrawerContent
	title={rule ? `Protection rule ${rule.name}` : 'New protection rule'}
	on:close={onClose}
>
	<div class="flex flex-col gap-6">
		<p class="text-xs text-secondary">A rule can take up to one minute to take effect.</p>

		<Label label="Name" required>
			<TextInput
				bind:value={name}
				error={shownNameError}
				inputProps={{
					placeholder: 'Enter rule name',
					disabled: isNameLocked
				}}
			/>
			{#if isNameLocked}
				<p class="text-2xs text-hint">
					Managed by the dev workspace pairing, which locates this rule by name. Its restrictions
					and bypassers can still be changed.
				</p>
			{:else if shownNameError}
				<p class="text-2xs text-red-500">{shownNameError}</p>
			{/if}
		</Label>

		<Section
			label="Bypassers"
			description="Groups and users who are exempt from the restrictions below."
			class="flex flex-col gap-4"
		>
			<Label label="Groups">
				<MultiSelect
					bind:value={selectedGroups}
					items={safeSelectItems(availableGroups)}
					placeholder="Select groups"
					reorderable={false}
				/>
			</Label>
			<Label label="Users">
				<MultiSelect
					bind:value={selectedUsers}
					items={safeSelectItems(availableUsers)}
					placeholder="Select users"
					reorderable={false}
				/>
			</Label>
		</Section>

		<Section label="Restrictions" class="flex flex-col gap-4">
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={disableDirectDeployment}
					options={{ right: 'Disable direct deployment' }}
				/>
				{@render description(
					'Users must use a fork or git branch to make changes. Direct edits are not allowed.'
				)}
			</div>
			<div class="flex flex-col gap-1">
				<Toggle bind:checked={disableFork} options={{ right: 'Disable workspace forking' }} />
				{@render description('Users cannot create forks of this workspace.')}
			</div>
			<div class="flex flex-col gap-1">
				<Toggle bind:checked={requireIsolatedForks} options={{ right: 'Require isolated forks' }} />
				{@render description(
					"Forks of this workspace cannot share its data: every object storage and ducklake stays isolated, and every data table is cloned instead of pointing at this workspace's database."
				)}
			</div>
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={restrictDeployToDeployers}
					options={{ right: 'Restrict deployment to wm_deployers' }}
				/>
				{@render description(
					'Only workspace admins and members of wm_deployers can deploy to this workspace. Non-deployers can still fork, browse, and request a review.'
				)}
			</div>
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={restrictAnonymousAppDeployment}
					options={{ right: 'Restrict public app access' }}
				/>
				{@render description(
					'Only workspace admins and bypassers can make an app publicly accessible without login (anonymous execution mode). Apps that are already public can still be redeployed.'
				)}
			</div>
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={restrictGuestAppDeployment}
					options={{ right: 'Restrict guest app access' }}
				/>
				{@render description(
					'Only workspace admins and bypassers can open an app to guests (anyone who signs in, with no workspace membership and no seat). Apps that already admit guests can still be redeployed.'
				)}
			</div>
			<div class="flex flex-col gap-1">
				<Toggle
					bind:checked={restrictPublicRunSharing}
					options={{ right: 'Restrict public run sharing' }}
				/>
				{@render description(
					'Only workspace admins and bypassers can mint a public link to a run (readable without login). The read-only link for workspace members stays available to everyone.'
				)}
			</div>
		</Section>

		<Section
			label="Roles in forks"
			description="A member forking this workspace keeps their role in the fork. Map a role to give its members another one in their forks, for example operator to developer. Only roles that may fork can use it."
			class="flex flex-col gap-2"
		>
			{#each forkRoleMapping as mapping, i (i)}
				<div class="flex items-center gap-2">
					<Select
						bind:value={mapping.from}
						items={availableRoles}
						placeholder="Role here"
						class="grow"
					/>
					<ArrowRight size={14} class="text-secondary shrink-0" />
					<Select
						bind:value={mapping.to}
						items={availableRoles}
						placeholder="Role in the fork"
						class="grow"
					/>
					<Button
						variant="subtle"
						unifiedSize="sm"
						iconOnly
						startIcon={{ icon: Trash2 }}
						aria-label="Remove mapping"
						onclick={() => forkRoleMapping.splice(i, 1)}
					/>
				</div>
			{/each}
			{#if mappingError}
				<p class="text-2xs text-red-500">{mappingError}</p>
			{/if}
			<div>
				<Button
					variant="default"
					unifiedSize="sm"
					startIcon={{ icon: Plus }}
					onclick={() => forkRoleMapping.push({ from: undefined, to: undefined })}
				>
					Add mapping
				</Button>
			</div>
		</Section>
	</div>

	{#snippet actions()}
		<Button
			variant="accent"
			unifiedSize="md"
			disabled={!canSave}
			loading={saving}
			startIcon={{ icon: Save }}
			on:click={isCreateMode ? create : save}
		>
			{isCreateMode ? 'Create' : 'Save'}
		</Button>
	{/snippet}
</DrawerContent>
