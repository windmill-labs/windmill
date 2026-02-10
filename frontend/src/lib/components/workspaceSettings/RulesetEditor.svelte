<script lang="ts">
	import { Section, Button, Badge } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { workspaceStore } from '$lib/stores'
	import { GroupService, UserService, WorkspaceService, type ProtectionRuleKind, type ProtectionRuleset } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clone } from '$lib/utils'
	import { untrack } from 'svelte'
	import { Save, X, Plus } from 'lucide-svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	interface Props {
		rule?: ProtectionRuleset
		existingNames?: string[]
		onUpdate?: () => void
	}

	let { rule, existingNames = [], onUpdate }: Props = $props()

	// Create mode vs Edit mode
	const isCreateMode = $derived(!rule)

	// Helper function to check if a rule is in the array
	const hasRule = (ruleKind: string) => rule?.rules?.includes(ruleKind as any) ?? false

	// Editable state
	let name = $state(rule?.name ?? '')
	let disableDirectDeployment = $state(hasRule('DisableDirectDeployment'))
	let disableFork = $state(hasRule('DisableWorkspaceForking'))
	let selectedGroups = $state<string[]>(rule?.bypass_groups?.map((g) => g.replace('g/', '')) ?? [])
	let selectedUsers = $state<string[]>(rule?.bypass_users?.map((u) => u.replace('u/', '')) ?? [])

	// Initial state for unsaved changes tracking
	let initialName = $state(rule?.name ?? '')
	let initialDisableDirectDeployment = $state(hasRule('DisableDirectDeployment'))
	let initialDisableFork = $state(hasRule('DisableWorkspaceForking'))
	let initialSelectedGroups = $state<string[]>(
		rule?.bypass_groups ? rule.bypass_groups.map((g) => g.replace('g/', '')) : []
	)
	let initialSelectedUsers = $state<string[]>(
		rule?.bypass_users ? rule.bypass_users.map((u) => u.replace('u/', '')) : []
	)

	// Available options
	let availableGroups = $state<string[]>([])
	let availableUsers = $state<string[]>([])

	// Temporary values for Select dropdowns
	let selectedGroupToAdd = $state<string | undefined>(undefined)
	let selectedUserToAdd = $state<string | undefined>(undefined)

	// Load available groups and users
	async function loadAvailableGroups() {
		const groups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
		availableGroups = groups
	}

	async function loadAvailableUsers() {
		const users = await UserService.listUsernames({ workspace: $workspaceStore! })
		availableUsers = users
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadAvailableGroups()
				loadAvailableUsers()
			})
		}
	})

	// Effect to add selected group
	$effect(() => {
		if (selectedGroupToAdd && !selectedGroups.includes(selectedGroupToAdd)) {
			selectedGroups = [...selectedGroups, selectedGroupToAdd]
			untrack(() => {
				selectedGroupToAdd = undefined // Reset for next selection
			})
		}
	})

	// Effect to add selected user
	$effect(() => {
		if (selectedUserToAdd && !selectedUsers.includes(selectedUserToAdd)) {
			selectedUsers = [...selectedUsers, selectedUserToAdd]
			untrack(() => {
				selectedUserToAdd = undefined // Reset for next selection
			})
		}
	})

	// Computed properties
	const hasUnsavedChanges = $derived(
		isCreateMode
			? name.trim() !== '' ||
					disableDirectDeployment ||
					disableFork ||
					selectedGroups.length > 0 ||
					selectedUsers.length > 0
			: name !== initialName ||
					disableDirectDeployment !== initialDisableDirectDeployment ||
					disableFork !== initialDisableFork ||
					JSON.stringify([...selectedGroups].sort()) !==
						JSON.stringify([...initialSelectedGroups].sort()) ||
					JSON.stringify([...selectedUsers].sort()) !== JSON.stringify([...initialSelectedUsers].sort())
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

	const canSave = $derived(!nameError && hasUnsavedChanges)

	function removeGroup(group: string) {
		selectedGroups = selectedGroups.filter((g) => g !== group)
	}

	function removeUser(user: string) {
		selectedUsers = selectedUsers.filter((u) => u !== user)
	}

	async function create() {
		if (!canSave || !$workspaceStore) return

		try {
			await WorkspaceService.createProtectionRule({
				workspace: $workspaceStore,
				requestBody: {
					name,
					rules: [
						...(disableDirectDeployment ? ['DisableDirectDeployment' as ProtectionRuleKind] : []),
						...(disableFork ? ['DisableWorkspaceForking' as ProtectionRuleKind] : []),
					],
					bypass_groups: selectedGroups,
					bypass_users: selectedUsers
				}
			})

			sendUserToast('Protection rule created successfully')
			onUpdate?.()
		} catch (error) {
			console.error('Failed to create protection rule:', error)
			sendUserToast('Failed to create protection rule', true)
		}
	}

	async function save() {
		if (!canSave || !$workspaceStore) return

		try {
			await WorkspaceService.updateProtectionRule({
				workspace: $workspaceStore,
				ruleName: initialName,
				requestBody: {
					rules: [
						...(disableDirectDeployment ? ['DisableDirectDeployment' as ProtectionRuleKind] : []),
						...(disableFork ? ['DisableWorkspaceForking' as ProtectionRuleKind] : []),
					],
					bypass_groups: selectedGroups,
					bypass_users: selectedUsers
				}
			})

			sendUserToast('Protection rule saved successfully')

			// Update initial state
			initialName = name
			initialDisableDirectDeployment = disableDirectDeployment
			initialDisableFork = disableFork
			initialSelectedGroups = clone(selectedGroups)
			initialSelectedUsers = clone(selectedUsers)

			onUpdate?.()
		} catch (error) {
			console.error('Failed to save protection rule:', error)
			sendUserToast('Failed to save protection rule', true)
		}
	}
</script>

<div class="flex flex-col gap-6 p-4">
	<!-- Name Section -->
	<span class="text-secondary text-sm">
		Keep in mind that rulesets can take up to one minute to take effect
	</span>
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
			<Label class="text-xs">Groups</Label>
			<Select
				bind:value={selectedGroupToAdd}
				items={safeSelectItems(availableGroups.filter((g) => !selectedGroups.includes(g)))}
				placeholder="Select groups..."
			/>
			{#if selectedGroups.length > 0}
				<div class="flex flex-wrap gap-2 mt-2">
					{#each selectedGroups as group (group)}
						<Badge color="blue" class="flex items-center gap-1">
							{group}
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
			<Label class="text-xs">Users</Label>
			<Select
				bind:value={selectedUserToAdd}
				items={safeSelectItems(availableUsers.filter((u) => !selectedUsers.includes(u)))}
				placeholder="Select users..."
			/>
			{#if selectedUsers.length > 0}
				<div class="flex flex-wrap gap-2 mt-2">
					{#each selectedUsers as user (user)}
						<Badge color="indigo" class="flex items-center gap-1">
							{user}
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
		description="Configure the rules that will be enforced"
		class="space-y-4"
	>
		<div class="flex flex-col gap-4">
			<!-- Disable Direct Deployment -->
			<div class="flex flex-col gap-2">
				<Toggle
					bind:checked={disableDirectDeployment}
					options={{
						right: 'Disable direct deployment'
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

		</div>
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
