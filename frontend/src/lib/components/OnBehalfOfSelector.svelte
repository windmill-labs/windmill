<script lang="ts" module>
	import { isTriggerOrScheduleKind } from 'windmill-utils-internal'

	export type OnBehalfOfChoice = 'target' | 'me' | 'custom' | undefined

	export interface OnBehalfOfDetails {
		email: string
		permissionedAs: string
	}

	/**
	 * Check if an item needs on_behalf_of selection.
	 * Shows the selector when the source item has an on_behalf_of value set.
	 */
	export function needsOnBehalfOfSelection(kind: string, sourceValue: string | undefined): boolean {
		const supported =
			kind === 'flow' ||
			kind === 'script' ||
			kind === 'app' ||
			kind === 'raw_app' ||
			kind === 'trigger' ||
			isTriggerOrScheduleKind(kind) ||
			// Only an agent's value carries an identity, so no other resource has a source value.
			kind === 'resource'
		if (!supported) return false
		return !!sourceValue
	}
</script>

<script lang="ts">
	import { Bot, Check, User as UserIcon, UserCog, Users, ExternalLink, Plus } from 'lucide-svelte'
	import MeltPopover from './meltComponents/Popover.svelte'
	import Portal from './Portal.svelte'
	import Modal from './common/modal/Modal.svelte'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { UserService, type User } from '$lib/gen'
	import TextInput from './text_input/TextInput.svelte'
	import { useDeployAs } from './access/deployAs.svelte'
	import {
		getPendingServiceAccount,
		PENDING_EMAIL_DOMAIN,
		useServiceAccounts,
		type ServiceAccount
	} from './access/serviceAccounts.svelte'
	import NewServiceAccountForm from './access/NewServiceAccountForm.svelte'
	import type { NeedsSource } from './access/detectNeeds'

	interface Props {
		targetWorkspace: string
		/** The target value: email for flows/scripts, permissioned_as (u/username) for triggers */
		targetValue: string | undefined
		selected: OnBehalfOfChoice
		/**
		 * Called when the user picks a choice.
		 * For 'custom', `details` contains both `email` and `permissionedAs` (u/{username}).
		 * Callers pick whichever format they need.
		 */
		onSelect: (choice: OnBehalfOfChoice, details?: OnBehalfOfDetails) => void
		kind: string
		canPreserve: boolean
		/** The value of the custom-selected user (for display) */
		customValue?: string | undefined
		/** When false, labels say "current" instead of "target" and modal text refers to "this workspace" */
		isDeployment?: boolean
		/**
		 * Folder default permissioned_as for this item path — resolved by the parent via
		 * `useFolderDefaultPermissionedAs`. When set and no explicit choice has been made,
		 * the selector preselects `'custom'` with this value and shows a "folder default"
		 * badge so the user sees where the value came from.
		 */
		folderDefault?: string | undefined
		/**
		 * Raises the popover and user picker above a `ConfirmationModal`, which renders at a
		 * z-index above the popover layer — without this they open behind the dialog hosting them.
		 */
		aboveConfirmationModal?: boolean
		/**
		 * Fires as the user picker opens and closes. A host that binds its own Enter/Escape handler
		 * must suspend it meanwhile: both handlers sit on `window`, where `stopPropagation` can't
		 * separate them, so Escape in the picker would also dismiss the host.
		 */
		onPickerOpenChange?: (open: boolean) => void
		/**
		 * `u/username` to label the "me" option with. Usernames are per-workspace, so the current
		 * one is the wrong name whenever the deploy targets a different workspace.
		 */
		myPermissionedAs?: string
		/** Without `canPreserve`, show the current user as plain text instead of a popover whose
		 *  only enabled option is "me". The host must treat that as the choice already made. */
		readOnlyWithoutPreserve?: boolean
		/** The item's path, which a new service account is named after and given read on. */
		itemPath?: string
		/** The item as its editor holds it, read for what a new service account needs. */
		needsSource?: () => NeedsSource | undefined
	}

	let {
		targetWorkspace,
		targetValue,
		selected,
		onSelect,
		kind,
		canPreserve,
		customValue,
		isDeployment = true,
		folderDefault = undefined,
		aboveConfirmationModal = false,
		onPickerOpenChange = undefined,
		myPermissionedAs = undefined,
		readOnlyWithoutPreserve = false,
		itemPath = undefined,
		needsSource = undefined
	}: Props = $props()

	const isTrigger = $derived(kind === 'trigger' || isTriggerOrScheduleKind(kind))

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

	/** Resolve a value to a display name, always showing u/username format */
	function resolveDisplayName(value: string | undefined): string | undefined {
		if (!value) return undefined
		if (value.startsWith('u/') || value.startsWith('g/')) return value
		if (value.endsWith(`@${PENDING_EMAIL_DOMAIN}`)) return `u/${value.split('@')[0]}`
		const username = users.find((u) => u.email === value)?.username
		return username ? `u/${username}` : value
	}

	let targetDisplayName = $derived(resolveDisplayName(targetValue))
	let customDisplayName = $derived(resolveDisplayName(customValue))
	let myDisplayName = $derived(
		myPermissionedAs ?? ($userStore?.username ? `u/${$userStore.username}` : undefined)
	)

	// Without `canPreserve`, the service accounts whose deployers list admits the user are still
	// theirs to pick or keep.
	const deployAs = useDeployAs(() => targetWorkspace)
	const canPick = $derived(canPreserve || deployAs.current.length > 0)
	function allowed(value: string | undefined): boolean {
		const name = resolveDisplayName(value)
		return canPreserve || (!!name && deployAs.current.includes(name))
	}

	const serviceAccounts = useServiceAccounts(() => targetWorkspace)
	const q = $derived(searchQuery.trim().toLowerCase())
	let people = $derived(
		users.filter(
			(u) =>
				!u.disabled &&
				!u.is_service_account &&
				canPreserve &&
				(!q ||
					u.username.toLowerCase().includes(q) ||
					u.email.toLowerCase().includes(q) ||
					(u.name?.toLowerCase()?.includes(q) ?? false))
		)
	)
	let accounts = $derived(
		serviceAccounts.current
			.filter(
				(a) =>
					(a.pending || canPreserve || deployAs.current.includes(`u/${a.username}`)) &&
					(!q || a.username.toLowerCase().includes(q))
			)
			.sort(
				(a, b) => Number(!!b.pending) - Number(!!a.pending) || a.username.localeCompare(b.username)
			)
	)

	// Service accounts are created at deploy time for the runnables a deploy dialog confirms; a
	// trigger's identity is set where it is saved, with no dialog to create one from.
	const offersCreate = $derived(
		kind === 'script' || kind === 'flow' || kind === 'app' || kind === 'raw_app'
	)
	const canCreate = $derived(!!$enterpriseLicense && !!$userStore?.is_admin)
	let mode: 'pick' | 'create' = $state('pick')
	let createName: string | undefined = $state()

	function startCreate(name?: string) {
		createName = name
		mode = 'create'
	}

	function isPending(value: string | undefined): boolean {
		return !!getPendingServiceAccount(targetWorkspace, resolveDisplayName(value))
	}

	// Preselect "target" when available and user has permission to preserve.
	// When no target is set but a folder default is, preselect that as a custom value.
	// We track `users.length` so the effect re-runs once the async user list loads.
	$effect(() => {
		if (selected !== undefined) return
		if (targetValue && allowed(targetValue)) {
			onSelect('target')
		} else if (folderDefault && allowed(folderDefault) && users.length > 0) {
			const username = folderDefault.startsWith('u/') ? folderDefault.slice(2) : folderDefault
			const email = users.find((u) => u.username === username)?.email ?? folderDefault
			onSelect('custom', { email, permissionedAs: folderDefault })
		}
	})

	const isFolderDefaultSelected = $derived(
		selected === 'custom' && folderDefault !== undefined && customValue === folderDefault
	)

	function openModal() {
		loadUsers()
		searchQuery = ''
		mode = 'pick'
		modalOpen = true
	}

	function selectUser(user: Pick<User, 'email' | 'username'> | ServiceAccount) {
		onSelect('custom', { email: user.email, permissionedAs: `u/${user.username}` })
		modalOpen = false
	}

	function isSelected(username: string, email: string): boolean {
		return selected === 'custom' && (customValue === `u/${username}` || customValue === email)
	}

	// Covers every close path — the X, the overlay, Escape — not just `selectUser`.
	$effect(() => {
		onPickerOpenChange?.(modalOpen)
	})

	let selectedDisplayName = $derived.by(() => {
		if (selected === 'target') return targetDisplayName
		if (selected === 'me') return myDisplayName
		if (selected === 'custom') return customDisplayName
		return undefined
	})
</script>

{#if readOnlyWithoutPreserve && !canPick}
	<span class="inline-flex items-center gap-1 text-xs text-secondary">
		<UserIcon class="w-4 h-4" />
		{myDisplayName}
	</span>
{:else}
	<MeltPopover
		placement="bottom"
		contentClasses={aboveConfirmationModal ? 'z-[10001]' : ''}
		on:openChange={(e) => e.detail && loadUsers()}
	>
		{#snippet trigger()}
			<span class="inline-flex items-center gap-1">
				<UserCog class="w-4 h-4 {selected ? 'text-green-500' : 'text-yellow-500'}" />
				{#if selectedDisplayName}
					<span class="text-xs truncate max-w-24">{selectedDisplayName}</span>
					{#if selected === 'custom' && isPending(customValue)}
						<span
							class="text-[10px] px-1 py-0.5 rounded bg-blue-50 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300"
							title="Created when you deploy">new</span
						>
					{/if}
				{/if}
				{#if isFolderDefaultSelected}
					<span
						class="text-[10px] px-1 py-0.5 rounded bg-blue-50 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300 border border-blue-200 dark:border-blue-900"
						title="From this folder's default_permissioned_as rule"
					>
						folder default
					</span>
				{/if}
			</span>
		{/snippet}
		{#snippet content({ close: closePopover })}
			<div class="p-3 flex flex-col gap-2 min-w-48">
				<div class="text-xs font-medium text-secondary mb-1">{label}</div>
				<!-- Target option -->
				{#if targetValue}
					<button
						class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover {!allowed(
							targetValue
						)
							? 'opacity-50 cursor-not-allowed'
							: ''}"
						disabled={!allowed(targetValue)}
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
					class="flex items-center gap-2 px-2 py-1.5 rounded text-left text-xs hover:bg-surface-hover {!canPick
						? 'opacity-50 cursor-not-allowed'
						: ''}"
					disabled={!canPick}
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
						<span>{canPreserve ? 'Pick from workspace' : 'Pick a service account'}&hellip;</span>
					{/if}
				</button>
			</div>
		{/snippet}
	</MeltPopover>
{/if}

<!-- User selection modal. Portalled: the modal positions itself with `fixed`, which resolves
     against the nearest transformed ancestor — inside a dialog card (which is transformed for
     its open transition) it would be laid out within that card instead of the viewport. -->
<Portal>
	<Modal
		title={mode === 'create' ? 'New service account' : 'Select a user'}
		bind:open={modalOpen}
		kind="X"
		minZIndex={aboveConfirmationModal ? 10001 : undefined}
	>
		{#if mode === 'create'}
			<div class="w-[32rem] max-w-full">
				<NewServiceAccountForm
					workspace={targetWorkspace}
					itemKind={kind === 'raw_app' ? 'app' : (kind as 'script' | 'flow' | 'app')}
					{itemPath}
					{needsSource}
					initialName={createName}
					onBack={() => (mode = 'pick')}
					onUse={(permissionedAs) => {
						onSelect('custom', {
							email: `${permissionedAs.slice(2)}@${PENDING_EMAIL_DOMAIN}`,
							permissionedAs
						})
						modalOpen = false
					}}
				/>
			</div>
		{:else}
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

				<TextInput
					bind:value={searchQuery}
					inputProps={{ placeholder: 'Search people and service accounts' }}
				/>

				<div class="max-h-72 overflow-y-auto border rounded">
					{#if accounts.length}
						<div class="px-3 pt-2 pb-1 text-2xs font-semibold text-secondary">Service accounts</div>
						{#each accounts as account (account.username)}
							<button
								class="w-full flex items-center gap-3 px-3 py-2 text-left text-sm hover:bg-surface-hover"
								onclick={() => selectUser(account)}
							>
								<Bot class="w-4 h-4 text-secondary flex-shrink-0" />
								<span class="font-medium truncate">u/{account.username}</span>
								{#if account.pending}
									<span class="text-2xs text-blue-700 dark:text-blue-300">new</span>
								{/if}
								<span class="ml-auto text-xs text-tertiary truncate">
									{account.folders.length
										? `reads ${account.folders.map((f) => `f/${f}`).join(', ')}`
										: 'no folder shared'}
								</span>
								{#if isSelected(account.username, account.email)}
									<Check class="w-4 h-4 text-green-500 flex-shrink-0" />
								{/if}
							</button>
						{/each}
					{/if}
					{#if people.length}
						<div class="px-3 pt-2 pb-1 text-2xs font-semibold text-secondary">People</div>
						{#each people as user (user.email)}
							<button
								class="w-full flex items-center gap-3 px-3 py-2 text-left text-sm hover:bg-surface-hover"
								onclick={() => selectUser(user)}
							>
								<UserIcon class="w-4 h-4 text-secondary flex-shrink-0" />
								<span class="font-medium truncate">u/{user.username}</span>
								<span class="ml-auto text-xs text-tertiary truncate">{user.email}</span>
								{#if isSelected(user.username, user.email)}
									<Check class="w-4 h-4 text-green-500 flex-shrink-0" />
								{/if}
							</button>
						{/each}
					{/if}
					{#if !accounts.length && !people.length}
						<div class="px-3 py-4 text-sm text-tertiary text-center">
							{#if !usersLoaded}
								Loading users&hellip;
							{:else if q}
								No one matches “{searchQuery.trim()}”
							{:else}
								No users found
							{/if}
						</div>
					{/if}
					{#if offersCreate}
						<div class="border-t">
							<button
								class="w-full flex items-center gap-2 px-3 py-2 text-left text-sm font-semibold {canCreate
									? 'text-accent hover:bg-surface-hover'
									: 'text-disabled cursor-not-allowed'}"
								disabled={!canCreate}
								onclick={() =>
									startCreate(!accounts.length && !people.length ? searchQuery.trim() : undefined)}
							>
								<Plus class="w-4 h-4" />
								<span class="grow">
									{#if canCreate && q && !accounts.length && !people.length}
										Create service account “{searchQuery.trim()}”
									{:else}
										New service account
									{/if}
								</span>
								{#if !$enterpriseLicense}
									<span
										class="text-2xs font-medium px-1.5 rounded bg-surface-secondary text-secondary"
										>EE</span
									>
								{/if}
							</button>
							{#if $enterpriseLicense && !canCreate}
								<div class="px-3 pb-2 pl-9 text-2xs text-secondary">
									Only workspace admins can create one.
								</div>
							{/if}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</Modal>
</Portal>
