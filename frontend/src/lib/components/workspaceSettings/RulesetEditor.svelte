<script lang="ts">
	import { Section, Alert, Button, Badge } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { enterpriseLicense, workspaceStore, userStore } from '$lib/stores'
	import { GroupService, UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clone } from '$lib/utils'
	import { untrack, createEventDispatcher } from 'svelte'
	import { Save, X, Plus } from 'lucide-svelte'

	interface Rule {
		name: string
		rules: {
			requireForkOrBranch: boolean
			disableFork: boolean
			disableMergeUI: boolean
			disableExecution: boolean
			adminsBypassDisabled: boolean
		}
		scope: {
			groups: string[]
			users: string[]
		}
	}

	interface Props {
		rule?: Rule
		existingNames?: string[]
	}

	let { rule, existingNames = [] }: Props = $props()

	const dispatch = createEventDispatcher()

	// Create mode vs Edit mode
	const isCreateMode = $derived(!rule)

	// Editable state
	let name = $state(rule?.name ?? '')
	let requireForkOrBranch = $state(rule?.rules.requireForkOrBranch ?? false)
	let disableFork = $state(rule?.rules.disableFork ?? false)
	let disableMergeUI = $state(rule?.rules.disableMergeUI ?? false)
	let disableExecution = $state(rule?.rules.disableExecution ?? false)
	let adminsBypassDisabled = $state(rule?.rules.adminsBypassDisabled ?? false)
	let selectedGroups = $state<string[]>(rule?.scope.groups ?? [])
	let selectedUsers = $state<string[]>(rule?.scope.users ?? [])

	// Initial state for unsaved changes tracking
	let initialName = $state(rule?.name ?? '')
	let initialRequireForkOrBranch = $state(rule?.rules.requireForkOrBranch ?? false)
	let initialDisableFork = $state(rule?.rules.disableFork ?? false)
	let initialDisableMergeUI = $state(rule?.rules.disableMergeUI ?? false)
	let initialDisableExecution = $state(rule?.rules.disableExecution ?? false)
	let initialAdminsBypassDisabled = $state(rule?.rules.adminsBypassDisabled ?? false)
	let initialSelectedGroups = $state<string[]>(rule?.scope.groups ? clone(rule.scope.groups) : [])
	let initialSelectedUsers = $state<string[]>(rule?.scope.users ? clone(rule.scope.users) : [])

	// Available options
	let availableGroups = $state<string[]>([])
	let availableUsers = $state<string[]>([])

	// Load available groups and users
	async function loadAvailableGroups() {
		const groups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
		availableGroups = groups.map((g) => `g/${g}`)
	}

	async function loadAvailableUsers() {
		const users = await UserService.listUsernames({ workspace: $workspaceStore! })
		availableUsers = users.map((u) => `u/${u}`)
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadAvailableGroups()
				loadAvailableUsers()
			})
		}
	})

	// Computed properties
	const hasUnsavedChanges = $derived(
		isCreateMode
			? name.trim() !== '' ||
					requireForkOrBranch ||
					disableFork ||
					disableMergeUI ||
					disableExecution ||
					adminsBypassDisabled ||
					selectedGroups.length > 0 ||
					selectedUsers.length > 0
			: name !== initialName ||
					requireForkOrBranch !== initialRequireForkOrBranch ||
					disableFork !== initialDisableFork ||
					disableMergeUI !== initialDisableMergeUI ||
					disableExecution !== initialDisableExecution ||
					adminsBypassDisabled !== initialAdminsBypassDisabled ||
					JSON.stringify([...selectedGroups].sort()) !==
						JSON.stringify([...initialSelectedGroups].sort()) ||
					JSON.stringify([...selectedUsers].sort()) !== JSON.stringify([...initialSelectedUsers].sort())
	)

	const scopeEmpty = $derived(selectedGroups.length === 0 && selectedUsers.length === 0)

	const nameError = $derived.by(() => {
		if (!name.trim()) return 'Name is required'
		if (isCreateMode) {
			if (existingNames.includes(name)) return 'Name already exists'
		} else {
			if (name !== initialName && existingNames.includes(name)) return 'Name already exists'
		}
		return undefined
	})

	const canSave = $derived(!nameError && hasUnsavedChanges)

	const enabledRulesCount = $derived(
		[requireForkOrBranch, disableFork, disableMergeUI, disableExecution, adminsBypassDisabled].filter(Boolean)
			.length
	)

	// Logical warnings (non-blocking)
	const hasLogicalWarnings = $derived(
		(disableFork && requireForkOrBranch) || (disableMergeUI && !disableFork)
	)

	function removeGroup(group: string) {
		selectedGroups = selectedGroups.filter((g) => g !== group)
	}

	function removeUser(user: string) {
		selectedUsers = selectedUsers.filter((u) => u !== user)
	}

	async function create() {
		if (!canSave) return

		const newRule: Rule = {
			name,
			rules: {
				requireForkOrBranch,
				disableFork,
				disableMergeUI,
				disableExecution,
				adminsBypassDisabled
			},
			scope: {
				groups: selectedGroups,
				users: selectedUsers
			}
		}

		// TODO: Implement backend call
		// await WorkspaceService.createWorkspaceRule({
		//   workspace: $workspaceStore!,
		//   requestBody: newRule
		// })

		sendUserToast('Protection rule created successfully')
		dispatch('update')
	}

	async function save() {
		if (!canSave) return

		const updatedRule: Rule = {
			name,
			rules: {
				requireForkOrBranch,
				disableFork,
				disableMergeUI,
				disableExecution,
				adminsBypassDisabled
			},
			scope: {
				groups: selectedGroups,
				users: selectedUsers
			}
		}

		// TODO: Implement backend call
		// await WorkspaceService.updateWorkspaceRule({
		//   workspace: $workspaceStore!,
		//   name: initialName,
		//   requestBody: updatedRule
		// })

		sendUserToast('Protection rule saved successfully')

		// Update initial state
		initialName = name
		initialRequireForkOrBranch = requireForkOrBranch
		initialDisableFork = disableFork
		initialDisableMergeUI = disableMergeUI
		initialDisableExecution = disableExecution
		initialAdminsBypassDisabled = adminsBypassDisabled
		initialSelectedGroups = clone(selectedGroups)
		initialSelectedUsers = clone(selectedUsers)

		dispatch('update')
	}
</script>

<div class="flex flex-col gap-6 p-4">
	<!-- Name Section -->
	<Section label="Rule Name" class="space-y-2">
		<TextInput
			size="md"
			bind:value={name}
			error={nameError}
			inputProps={{
				placeholder: 'Enter rule name'
			}}
		/>
		{#if nameError}
			<div class="text-xs text-red-600">{nameError}</div>
		{/if}
	</Section>

	<!-- Bypass Permissions Section -->
	<Section
		label="Bypass Permissions"
		description="Select the groups and/or users who can bypass the restrictions defined in this rule. These users will be exempt from the rules configured below."
		class="space-y-4"
	>
		<!-- Groups -->
		<div class="flex flex-col gap-2">
			<Label>Groups</Label>
			<Select
				value={undefined}
				on:change={(e) => {
					const value = e.detail
					if (value && !selectedGroups.includes(value)) {
						selectedGroups = [...selectedGroups, value]
					}
				}}
				items={availableGroups.filter((g) => !selectedGroups.includes(g))}
				placeholder="Select groups..."
			/>
			{#if selectedGroups.length > 0}
				<div class="flex flex-wrap gap-2 mt-2">
					{#each selectedGroups as group (group)}
						<Badge color="blue" class="flex items-center gap-1">
							{group.replace('g/', '')}
							<button type="button" onclick={() => removeGroup(group)} class="ml-1 hover:text-red-600">
								<X size={14} />
							</button>
						</Badge>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Users -->
		<div class="flex flex-col gap-2">
			<Label>Users</Label>
			<Select
				value={undefined}
				on:change={(e) => {
					const value = e.detail
					if (value && !selectedUsers.includes(value)) {
						selectedUsers = [...selectedUsers, value]
					}
				}}
				items={availableUsers.filter((u) => !selectedUsers.includes(u))}
				placeholder="Select users..."
			/>
			{#if selectedUsers.length > 0}
				<div class="flex flex-wrap gap-2 mt-2">
					{#each selectedUsers as user (user)}
						<Badge color="indigo" class="flex items-center gap-1">
							{user.replace('u/', '')}
							<button type="button" onclick={() => removeUser(user)} class="ml-1 hover:text-red-600">
								<X size={14} />
							</button>
						</Badge>
					{/each}
				</div>
			{/if}
		</div>
	</Section>

	<!-- Protection Rules Section -->
	<Section
		label="Protection Rules"
		description="Configure the rules that will be enforced for users and groups in the scope. {enabledRulesCount}/5 rules enabled."
		class="space-y-4"
	>
		<div class="flex flex-col gap-4">
			<!-- Require Fork or Branch -->
			<div class="flex flex-col gap-2">
				<Toggle
					bind:checked={requireForkOrBranch}
					options={{
						right: 'Require fork or git branch for changes'
					}}
				/>
				<div class="text-xs text-secondary ml-6">
					Users must use a fork or git branch to make changes. Direct edits are not allowed.
				</div>
			</div>

			<!-- Disable Fork -->
			<div class="flex flex-col gap-2">
				<Toggle
					bind:checked={disableFork}
					options={{
						right: 'Disable workspace forking'
					}}
				/>
				<div class="text-xs text-secondary ml-6">Users cannot create forks of this workspace.</div>
			</div>

			<!-- Disable Merge UI -->
			<div class="flex flex-col gap-2">
				<Toggle
					bind:checked={disableMergeUI}
					options={{
						right: 'Disable merge UI for forks'
					}}
				/>
				<div class="text-xs text-secondary ml-6">
					Users cannot deploy fork changes through the web UI. Merges must be done through external
					processes such as a PR on the Git Sync repo.
				</div>
			</div>

			<!-- Disable Execution -->
			<div class="flex flex-col gap-2">
				<Toggle
					bind:checked={disableExecution}
					options={{
						right: 'Disable manual script and flow execution'
					}}
				/>
				<div class="text-xs text-secondary ml-6">
					Users cannot execute scripts or flows. Useful for read-only or review workspaces.
				</div>
			</div>

			<!-- Admin Bypass -->
			<div class="flex flex-col gap-2">
				<Toggle
					bind:checked={adminsBypassDisabled}
					options={{
						right: 'Disable admin bypass for these rules'
					}}
				/>
				<div class="text-xs text-secondary ml-6">
					Even workspace admins must follow these rules. Use for strict governance requirements.
				</div>
			</div>
		</div>

		{#if hasLogicalWarnings}
			<Alert type="info" title="Configuration notice">
				{#if disableFork && requireForkOrBranch}
					Both "Disable workspace forking" and "Require fork or git branch" are enabled. Users will
					not be able to make changes since they cannot fork and changes must be made through a
					fork.
				{/if}
				{#if disableMergeUI && !disableFork}
					"Disable merge UI" is enabled but "Disable workspace forking" is not. Users can still
					create forks but cannot merge them through the UI.
				{/if}
			</Alert>
		{/if}
	</Section>

	<!-- Actions -->
	<div class="flex items-center gap-4 pt-4 border-t">
		<Button
			variant="accent"
			unifiedSize="md"
			disabled={!canSave}
			on:click={isCreateMode ? create : save}
			startIcon={{ icon: isCreateMode ? Plus : Save }}
		>
			{isCreateMode ? 'Create Rule' : 'Save Rule'}
		</Button>

		{#if hasUnsavedChanges && !isCreateMode}
			<span class="text-xs text-secondary">You have unsaved changes</span>
		{/if}
	</div>
</div>
