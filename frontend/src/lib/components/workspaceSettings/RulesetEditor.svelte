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
	import { Save } from 'lucide-svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	interface Props {
		rule?: ProtectionRuleset
		existingNames?: string[]
		onUpdate?: () => void
		onClose?: () => void
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
	let selectedGroups = $state<string[]>(
		untrack(() => rule)?.bypass_groups?.map((g) => g.replace('g/', '')) ?? []
	)
	let selectedUsers = $state<string[]>(
		untrack(() => rule)?.bypass_users?.map((u) => u.replace('u/', '')) ?? []
	)

	// Initial state for unsaved changes tracking
	let initialName = $state(untrack(() => rule)?.name ?? '')
	let initialDisableDirectDeployment = $state(hasRule('DisableDirectDeployment'))
	let initialDisableFork = $state(hasRule('DisableWorkspaceForking'))
	let initialRestrictDeployToDeployers = $state(hasRule('RestrictDeployToDeployers'))
	let initialRestrictAnonymousAppDeployment = $state(hasRule('RestrictAnonymousAppDeployment'))
	let initialRestrictPublicRunSharing = $state(hasRule('RestrictPublicRunSharing'))
	let initialRestrictGuestAppDeployment = $state(hasRule('RestrictGuestAppDeployment'))
	let initialSelectedGroups = $state<string[]>(
		untrack(() => rule)?.bypass_groups
			? untrack(() => rule)!.bypass_groups.map((g) => g.replace('g/', ''))
			: []
	)
	let initialSelectedUsers = $state<string[]>(
		untrack(() => rule)?.bypass_users
			? untrack(() => rule)!.bypass_users.map((u) => u.replace('u/', ''))
			: []
	)

	// Available options
	let availableGroups = $state<string[]>([])
	let availableUsers = $state<string[]>([])

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
					disableDirectDeployment ||
					disableFork ||
					restrictDeployToDeployers ||
					restrictAnonymousAppDeployment ||
					restrictPublicRunSharing ||
					restrictGuestAppDeployment ||
					selectedGroups.length > 0 ||
					selectedUsers.length > 0
			: name !== initialName ||
					disableDirectDeployment !== initialDisableDirectDeployment ||
					disableFork !== initialDisableFork ||
					restrictDeployToDeployers !== initialRestrictDeployToDeployers ||
					restrictAnonymousAppDeployment !== initialRestrictAnonymousAppDeployment ||
					restrictPublicRunSharing !== initialRestrictPublicRunSharing ||
					restrictGuestAppDeployment !== initialRestrictGuestAppDeployment ||
					JSON.stringify([...selectedGroups].sort()) !==
						JSON.stringify([...initialSelectedGroups].sort()) ||
					JSON.stringify([...selectedUsers].sort()) !==
						JSON.stringify([...initialSelectedUsers].sort())
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
							: [])
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
							: [])
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
			initialRestrictDeployToDeployers = restrictDeployToDeployers
			initialRestrictAnonymousAppDeployment = restrictAnonymousAppDeployment
			initialRestrictPublicRunSharing = restrictPublicRunSharing
			initialRestrictGuestAppDeployment = restrictGuestAppDeployment
			initialSelectedGroups = clone(selectedGroups)
			initialSelectedUsers = clone(selectedUsers)

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
