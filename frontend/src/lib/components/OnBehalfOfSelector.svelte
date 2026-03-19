<script lang="ts" module>
	export type OnBehalfOfChoice = 'target' | 'me' | 'custom' | undefined

	/**
	 * Check if an item needs on_behalf_of selection.
	 * Shows the selector when the source item has an on_behalf_of value set.
	 */
	export function needsOnBehalfOfSelection(kind: string, sourceValue: string | undefined): boolean {
		if (kind !== 'flow' && kind !== 'script' && kind !== 'app' && kind !== 'trigger') return false
		return !!sourceValue
	}
</script>

<script lang="ts">
	import { Check, UserCog, Users, ExternalLink } from 'lucide-svelte'
	import MeltPopover from './meltComponents/Popover.svelte'
	import Modal from './common/modal/Modal.svelte'
	import { userStore } from '$lib/stores'
	import { UserService, type User } from '$lib/gen'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		targetWorkspace: string
		/** The target value: email for flows/scripts, permissioned_as (u/username) for triggers */
		targetValue: string | undefined
		selected: OnBehalfOfChoice
		/**
		 * Called when the user picks a choice. `value` is the resolved value to use for deployment:
		 * - email for flows/scripts/apps
		 * - permissioned_as (u/username) for triggers/schedules
		 */
		onSelect: (choice: OnBehalfOfChoice, value?: string) => void
		kind: string
		canPreserve: boolean
		/** The value of the custom-selected user (for display) */
		customValue?: string | undefined
		/** When false, labels say "current" instead of "target" and modal text refers to "this workspace" */
		isDeployment?: boolean
	}

	let {
		targetWorkspace,
		targetValue,
		selected,
		onSelect,
		kind,
		canPreserve,
		customValue,
		isDeployment = true
	}: Props = $props()

	const isTrigger = $derived(kind === 'trigger')

	let label = $derived(
		isTrigger
			? 'Set the user this will be permissioned as:'
			: 'Set the user this will be run on behalf of:'
	)

	let users = $state<User[]>([])
	let usersLoaded = $state(false)
	let modalOpen = $state(false)
	let searchQuery = $state('')

	async function loadUsers() {
		if (usersLoaded) return
		try {
			users = await UserService.listUsers({ workspace: targetWorkspace })
		} catch {
			users = []
		}
		usersLoaded = true
	}

	// Fetch users eagerly so we can resolve usernames for display
	loadUsers()

	/** Format a username for display: u/username for triggers, plain username otherwise */
	function formatUsername(username: string | undefined): string | undefined {
		if (!username) return undefined
		return isTrigger ? `u/${username}` : username
	}

	/** Resolve a value to a display name */
	function resolveDisplayName(value: string | undefined): string | undefined {
		if (!value) return undefined
		// If already in permissioned_as format, show as-is
		if (value.startsWith('u/') || value.startsWith('g/')) return value
		// Try to resolve email to username
		const username = users.find((u) => u.email === value)?.username
		return formatUsername(username) ?? value
	}

	let targetDisplayName = $derived(resolveDisplayName(targetValue))
	let customDisplayName = $derived(resolveDisplayName(customValue))
	let myDisplayName = $derived(formatUsername($userStore?.username))

	let activeUsers = $derived(users.filter((u) => !u.disabled))
	let filteredUsers = $derived(
		searchQuery
			? activeUsers.filter((u) => {
					const q = searchQuery.toLowerCase()
					return (
						u.username.toLowerCase().includes(q) ||
						u.email.toLowerCase().includes(q) ||
						(u.name?.toLowerCase()?.includes(q) ?? false)
					)
				})
			: activeUsers
	)

	// Preselect "target" when available and user has permission to preserve
	$effect(() => {
		if (selected === undefined && targetValue && canPreserve) {
			onSelect('target')
		}
	})

	function openModal() {
		loadUsers()
		searchQuery = ''
		modalOpen = true
	}

	function selectUser(user: User) {
		// Return the right format: u/username for triggers, email for flows/scripts
		const value = isTrigger ? `u/${user.username}` : user.email
		onSelect('custom', value)
		modalOpen = false
	}

	let selectedDisplayName = $derived.by(() => {
		if (selected === 'target') return targetDisplayName
		if (selected === 'me') return myDisplayName
		if (selected === 'custom') return customDisplayName
		return undefined
	})
</script>

<MeltPopover placement="bottom" on:openChange={(e) => e.detail && loadUsers()}>
	{#snippet trigger()}
		<span class="inline-flex items-center gap-1">
			<UserCog class="w-4 h-4 {selected ? 'text-green-500' : 'text-yellow-500'}" />
			{#if selectedDisplayName}
				<span class="text-xs truncate max-w-24">{selectedDisplayName}</span>
			{/if}
		</span>
	{/snippet}
	{#snippet content({ close: closePopover })}
		<div class="p-3 flex flex-col gap-2 min-w-48">
			<div class="text-xs font-medium text-secondary mb-1">{label}</div>
			<!-- Target option -->
			{#if targetValue}
				<button
					class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover {!canPreserve
						? 'opacity-50 cursor-not-allowed'
						: ''}"
					disabled={!canPreserve}
					onclick={() => onSelect('target')}
				>
					<Check class="w-3 h-3 {selected === 'target' ? 'opacity-100' : 'opacity-0'}" />
					<span class="truncate max-w-40">{targetDisplayName}</span>
					<span class="text-xs text-tertiary">{isDeployment ? '(target)' : '(current)'}</span>
				</button>
			{/if}
			<!-- Me option -->
			<button
				class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover"
				onclick={() => onSelect('me')}
			>
				<Check class="w-3 h-3 {selected === 'me' ? 'opacity-100' : 'opacity-0'}" />
				<span class="truncate max-w-40">{myDisplayName}</span>
				<span class="text-xs text-tertiary">(me)</span>
			</button>
			<!-- Custom / Pick from workspace -->
			<button
				class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover {!canPreserve
					? 'opacity-50 cursor-not-allowed'
					: ''}"
				disabled={!canPreserve}
				onclick={() => {
					closePopover()
					openModal()
				}}
			>
				{#if selected === 'custom' && customDisplayName}
					<Check class="w-3 h-3 opacity-100" />
					<span class="truncate max-w-40">{customDisplayName}</span>
					<span class="text-xs text-tertiary">(custom)</span>
				{:else}
					<Check class="w-3 h-3 opacity-0" />
					<Users class="w-3 h-3 text-tertiary" />
					<span>Pick from workspace&hellip;</span>
				{/if}
			</button>
		</div>
	{/snippet}
</MeltPopover>

<!-- User selection modal -->
<Modal title="Select a user" bind:open={modalOpen} kind="X">
	<div class="flex flex-col gap-4">
		<div class="text-xs text-secondary">
			{#if isTrigger}
				Choose the user this trigger will be permissioned as {isDeployment
					? 'in the target workspace'
					: 'in this workspace'}. The selected user's permissions will be used when the trigger
				fires.
			{:else}
				Choose the user this {kind} will run on behalf of {isDeployment
					? 'in the target workspace'
					: 'in this workspace'}. The selected user's permissions will be used when executing.
			{/if}
			<a
				href="https://www.windmill.dev/docs/core_concepts/roles_and_permissions"
				target="_blank"
				rel="noopener noreferrer"
				class="text-blue-500 hover:underline inline-flex items-center gap-0.5"
			>
				Learn more
				<ExternalLink class="w-3 h-3" />
			</a>
		</div>

		<TextInput bind:value={searchQuery} inputProps={{ placeholder: 'Search users...' }} />

		<div class="max-h-60 overflow-y-auto border rounded">
			{#each filteredUsers as user (user.email)}
				<button
					class="w-full flex items-center gap-3 px-3 py-2 text-left text-sm hover:bg-surface-hover border-b last:border-b-0"
					onclick={() => selectUser(user)}
				>
					<div class="flex flex-col min-w-0">
						<span class="font-medium truncate"
							>{isTrigger ? `u/${user.username}` : user.username}</span
						>
						<span class="text-xs text-tertiary truncate">{user.email}</span>
					</div>
					{#if customValue === (isTrigger ? `u/${user.username}` : user.email) && selected === 'custom'}
						<Check class="w-4 h-4 text-green-500 ml-auto flex-shrink-0" />
					{/if}
				</button>
			{:else}
				<div class="px-3 py-4 text-sm text-tertiary text-center">
					{#if !usersLoaded}
						Loading users&hellip;
					{:else}
						No users found
					{/if}
				</div>
			{/each}
		</div>
	</div>
</Modal>
