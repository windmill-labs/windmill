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
	import { Bot, ChevronDown, PlusIcon, User as UserIcon, UserCog, Users } from 'lucide-svelte'
	import Select from './select/Select.svelte'
	import { Drawer, DrawerContent } from './common'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { UserService, type User } from '$lib/gen'
	import {
		getPendingServiceAccount,
		PENDING_EMAIL_DOMAIN,
		useServiceAccounts
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
		/** When false, the saved value is labelled "current" instead of "target" */
		isDeployment?: boolean
		/**
		 * Folder default permissioned_as for this item path — resolved by the parent via
		 * `useFolderDefaultPermissionedAs`. When set and no explicit choice has been made,
		 * the selector preselects `'custom'` with this value and shows a "folder default"
		 * badge so the user sees where the value came from.
		 */
		folderDefault?: string | undefined
		/**
		 * Raises the list and the creation drawer above a `ConfirmationModal`, which renders at a
		 * z-index above both — without this they open behind the dialog hosting them.
		 */
		aboveConfirmationModal?: boolean
		/**
		 * Fires as the list or the creation drawer opens and closes. A host that binds its own
		 * Enter/Escape handler must suspend it meanwhile: both handlers sit on `window`, where
		 * `stopPropagation` can't separate them, so Escape in the picker would also dismiss the host.
		 */
		onPickerOpenChange?: (open: boolean) => void
		/**
		 * `u/username` to label the "me" option with. Usernames are per-workspace, so the current
		 * one is the wrong name whenever the deploy targets a different workspace.
		 */
		myPermissionedAs?: string
		/** Without `canPreserve`, show the current user as plain text instead of a list whose
		 *  only enabled option is "me". The host must treat that as the choice already made. */
		readOnlyWithoutPreserve?: boolean
		/** The item's path, which a new service account is named after and given read on. */
		itemPath?: string
		/** The item as its editor holds it, read for what a new service account needs. */
		needsSource?: () => NeedsSource | undefined
		/** Offer to create a service account. Only a host whose deploy creates the pending account
		 *  may pass it: elsewhere the item would be deployed as an account that never exists. */
		offerCreate?: boolean
		size?: 'sm' | 'md'
		/** Width of the input, e.g. a fixed one in a table row so the row doesn't reflow. */
		class?: string
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
		needsSource = undefined,
		offerCreate = false,
		size = 'md',
		class: className = 'w-56'
	}: Props = $props()

	let users = $state<User[]>([])
	let usersLoaded = $state(false)

	async function loadUsers() {
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

	const serviceAccounts = useServiceAccounts(() => targetWorkspace)

	type Row = {
		value: string
		label: string
		subtitle?: string
		disabled?: boolean
		group: string
		kind: 'account' | 'person' | 'group'
		email?: string
		pending?: boolean
	}

	const GROUP_OF: Record<Row['kind'], string> = {
		account: 'Service accounts',
		person: 'People',
		group: 'Groups'
	}
	const GROUP_ORDER = Object.values(GROUP_OF)

	// One row per user: the saved value and "me" are tags on the account or person row when it is
	// listed, and rows of their own, at the top of the group of what they name, when it is not.
	let rows: Row[] = $derived.by(() => {
		const tags = new Map<string, string[]>()
		const tag = (name: string | undefined, t: string) =>
			name && tags.set(name, [...(tags.get(name) ?? []), t])
		if (targetValue) tag(targetDisplayName, isDeployment ? '(target)' : '(current)')
		tag(myDisplayName, '(me)')

		const listed: Omit<Row, 'label'>[] = [
			...serviceAccounts.current
				.filter((a) => a.pending || canPreserve)
				.sort(
					(a, b) =>
						Number(!!b.pending) - Number(!!a.pending) || a.username.localeCompare(b.username)
				)
				.map((a) => ({
					value: `u/${a.username}`,
					group: 'Service accounts',
					kind: 'account' as const,
					email: a.email,
					pending: a.pending,
					subtitle: a.folders.length
						? `reads ${a.folders.map((f) => `f/${f}`).join(', ')}`
						: 'no folder shared'
				})),
			...(canPreserve ? users : [])
				.filter((u) => !u.disabled && !u.is_service_account)
				.map((u) => ({
					value: `u/${u.username}`,
					group: 'People',
					kind: 'person' as const,
					email: u.email,
					subtitle: u.email
				}))
		]
		const listedNames = new Set(listed.map((r) => r.value))
		const own: Omit<Row, 'label'>[] = [...tags.keys()]
			.filter((name) => !listedNames.has(name))
			.map((name) => {
				const rowKind: Row['kind'] = name.startsWith('g/')
					? 'group'
					: serviceAccounts.current.some((a) => `u/${a.username}` === name)
						? 'account'
						: 'person'
				return {
					value: name,
					group: GROUP_OF[rowKind],
					kind: rowKind,
					disabled: name !== myDisplayName && !canPreserve
				}
			})
		// Select draws a header wherever the group changes, so each group's rows must be contiguous.
		return [...own, ...listed]
			.sort((a, b) => GROUP_ORDER.indexOf(a.group) - GROUP_ORDER.indexOf(b.group))
			.map((r) => ({
				...r,
				label: [r.value, ...(tags.get(r.value) ?? [])].join(' ')
			}))
	})
	let rowsByValue = $derived(new Map(rows.map((r) => [r.value, r])))

	function pick(value: string | undefined) {
		if (!value) return
		if (targetValue && value === targetDisplayName && canPreserve) {
			onSelect('target')
		} else if (value === myDisplayName) {
			onSelect('me')
		} else {
			const email = rowsByValue.get(value)?.email
			if (email) onSelect('custom', { email, permissionedAs: value })
		}
	}

	// Service accounts are created at deploy time for the runnables a deploy dialog confirms; a
	// trigger's identity is set where it is saved, with no dialog to create one from.
	const offersCreate = $derived(
		offerCreate && (kind === 'script' || kind === 'flow' || kind === 'app' || kind === 'raw_app')
	)
	const canCreate = $derived(!!$enterpriseLicense && !!$userStore?.is_admin)

	let filterText = $state('')
	let selectOpen = $state(false)
	let drawerOpen = $state(false)
	let createName: string | undefined = $state()

	function setSelectOpen(open: boolean) {
		selectOpen = open
		onPickerOpenChange?.(selectOpen || drawerOpen)
	}
	function setDrawerOpen(open: boolean) {
		drawerOpen = open
		onPickerOpenChange?.(selectOpen || drawerOpen)
	}

	function startCreate() {
		createName = filterText.trim() || undefined
		// Cleared first, as Select's own pick does: Select reopens the list while a search is typed.
		filterText = ''
		setDrawerOpen(true)
		setSelectOpen(false)
	}

	let noMatchingItems = $derived(
		!!filterText && !rows.some((r) => r.label.toLowerCase().includes(filterText.toLowerCase()))
	)

	function handleSelectKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && selectOpen && noMatchingItems && offersCreate && canCreate) {
			e.preventDefault()
			startCreate()
		}
	}

	function isPending(value: string | undefined): boolean {
		return !!getPendingServiceAccount(targetWorkspace, resolveDisplayName(value))
	}

	// Preselect "target" when available and user has permission to preserve.
	// When no target is set but a folder default is, preselect that as a custom value.
	// We track `users.length` so the effect re-runs once the async user list loads.
	$effect(() => {
		if (selected !== undefined) return
		if (targetValue && canPreserve) {
			onSelect('target')
		} else if (folderDefault && canPreserve && users.length > 0) {
			const username = folderDefault.startsWith('u/') ? folderDefault.slice(2) : folderDefault
			const email = users.find((u) => u.username === username)?.email ?? folderDefault
			onSelect('custom', { email, permissionedAs: folderDefault })
		}
	})

	const isFolderDefaultSelected = $derived(
		selected === 'custom' && folderDefault !== undefined && customValue === folderDefault
	)

	let selectedDisplayName = $derived.by(() => {
		if (selected === 'target') return targetDisplayName
		if (selected === 'me') return myDisplayName
		if (selected === 'custom') return customDisplayName
		return undefined
	})
</script>

{#snippet statusIcon()}
	<UserCog size={14} class={selected ? 'text-green-500' : 'text-yellow-500'} />
{/snippet}

{#if readOnlyWithoutPreserve && !canPreserve}
	<span class="inline-flex items-center gap-1 text-xs text-secondary">
		<UserIcon class="w-4 h-4" />
		{myDisplayName}
	</span>
{:else}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="inline-flex items-center gap-2" role="group" onkeydown={handleSelectKeydown}>
		<Select
			items={rows}
			bind:value={() => selectedDisplayName, pick}
			bind:filterText
			bind:open={() => selectOpen, setSelectOpen}
			groupBy={(r) => r.group}
			loading={!usersLoaded}
			noItemsMsg="No user or service account found"
			placeholder="Select a user"
			{size}
			class={className}
			dropdownClass={aboveConfirmationModal ? 'z-[10001]' : undefined}
			RightIcon={ChevronDown}
			inputLeadingSnippet={statusIcon}
		>
			{#snippet startSnippet({ item })}
				{@const rowKind = rowsByValue.get(item.value)?.kind}
				{@const Icon = rowKind === 'account' ? Bot : rowKind === 'group' ? Users : UserIcon}
				<Icon size={14} class="shrink-0 text-secondary" />
			{/snippet}
			{#snippet endSnippet({ item })}
				{#if rowsByValue.get(item.value)?.pending}
					<span class="text-2xs text-blue-700 dark:text-blue-300">new</span>
				{/if}
			{/snippet}
			{#snippet bottomSnippet()}
				{#if offersCreate}
					<button
						class="sticky py-2 px-4 w-full text-left text-xs font-medium flex items-center justify-center gap-2 border-t border-border-light {canCreate
							? 'hover:bg-surface-hover'
							: 'text-disabled cursor-not-allowed'} {noMatchingItems && canCreate
							? 'bg-surface-hover'
							: ''}"
						disabled={!canCreate}
						onclick={startCreate}
					>
						<PlusIcon class="inline" size={16} />
						Create service account
						{#if !$enterpriseLicense}
							<span class="text-2xs font-medium px-1.5 rounded bg-surface-secondary text-secondary"
								>EE</span
							>
						{/if}
					</button>
					{#if $enterpriseLicense && !canCreate}
						<div class="px-4 pb-2 text-center text-2xs text-secondary">
							Only workspace admins can create one.
						</div>
					{/if}
				{/if}
			{/snippet}
		</Select>
		{#if selected === 'custom' && isPending(customValue)}
			<span
				class="text-[10px] px-1 py-0.5 rounded bg-blue-50 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300"
				title="Created when you deploy">new</span
			>
		{/if}
		{#if isFolderDefaultSelected}
			<span
				class="text-[10px] px-1 py-0.5 rounded bg-blue-50 dark:bg-blue-950/50 text-blue-700 dark:text-blue-300 border border-blue-200 dark:border-blue-900 whitespace-nowrap"
				title="From this folder's default_permissioned_as rule"
			>
				folder default
			</span>
		{/if}
	</div>

	{#if offersCreate}
		<Drawer
			bind:open={() => drawerOpen, setDrawerOpen}
			size="600px"
			minZIndex={aboveConfirmationModal ? 10001 : undefined}
		>
			<DrawerContent title="New service account" on:close={() => setDrawerOpen(false)}>
				{#if drawerOpen}
					<NewServiceAccountForm
						workspace={targetWorkspace}
						itemKind={kind === 'raw_app' ? 'app' : (kind as 'script' | 'flow' | 'app')}
						{itemPath}
						{needsSource}
						initialName={createName}
						onBack={() => setDrawerOpen(false)}
						onUse={(permissionedAs) => {
							onSelect('custom', {
								email: `${permissionedAs.slice(2)}@${PENDING_EMAIL_DOMAIN}`,
								permissionedAs
							})
							setDrawerOpen(false)
						}}
					/>
				{/if}
			</DrawerContent>
		</Drawer>
	{/if}
{/if}
