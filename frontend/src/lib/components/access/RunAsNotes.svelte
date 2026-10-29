<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import {
		canDeployAs,
		canPreserveOnBehalfOf,
		WM_DEPLOYERS_GROUP
	} from '$lib/components/itemSettings/onBehalfOf'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { Bot, Folder, Users } from 'lucide-svelte'
	import { useDeployAs } from './deployAs.svelte'
	import type { NeedsSource } from './detectNeeds'
	import NewServiceAccountForm from './NewServiceAccountForm.svelte'
	import {
		getPendingServiceAccount,
		PENDING_EMAIL_DOMAIN,
		useServiceAccounts
	} from './serviceAccounts.svelte'

	let {
		kind,
		targetWorkspace,
		itemPath,
		needsSource,
		runsAs,
		isNew,
		savedPermissionedAs,
		onUseAccount,
		onCreatingChange
	}: {
		kind: 'script' | 'flow' | 'app'
		targetWorkspace: string
		itemPath?: string
		needsSource?: () => NeedsSource | undefined
		/** `u/<username>` the deploy leaves the item running as; unset when it runs as its caller. */
		runsAs: string | undefined
		isNew: boolean
		savedPermissionedAs: string | undefined
		onUseAccount: (username: string, email: string) => void
		/** See `OnBehalfOfSelector`'s `onPickerOpenChange`. */
		onCreatingChange?: (open: boolean) => void
	} = $props()

	const operatingUser = useOperatingUser()
	const me = $derived(
		operatingUser.current?.username ? `u/${operatingUser.current.username}` : undefined
	)
	const deployAs = useDeployAs(() => targetWorkspace)
	const serviceAccounts = useServiceAccounts(() => targetWorkspace)

	const pending = $derived(getPendingServiceAccount(targetWorkspace, runsAs))
	const itemFolder = $derived(
		itemPath?.startsWith('f/') ? itemPath.split('/')[1] || undefined : undefined
	)
	const canPick = $derived(
		canPreserveOnBehalfOf(operatingUser.current) || deployAs.current.length > 0
	)
	const canCreate = $derived(!!$enterpriseLicense && !!operatingUser.current?.is_admin)
	// An account that already reads the item's folder, which the deployer may deploy as.
	const folderAccount = $derived(
		itemFolder
			? serviceAccounts.current.find(
					(a) =>
						!a.pending &&
						a.folders.includes(itemFolder) &&
						canDeployAs(operatingUser.current, `u/${a.username}`, deployAs.current)
				)
			: undefined
	)
	const suggestsAccount = $derived(
		!!$enterpriseLicense &&
			!!itemFolder &&
			!!me &&
			runsAs === me &&
			((canPick && !!folderAccount) || canCreate)
	)
	// The deployed version runs as a service account this deployer can't keep: the deploy takes it
	// off and runs the item as them instead.
	const dropsAccount = $derived(
		!isNew &&
			!!savedPermissionedAs &&
			runsAs !== savedPermissionedAs &&
			!canDeployAs(operatingUser.current, savedPermissionedAs, deployAs.current) &&
			serviceAccounts.current.some((a) => `u/${a.username}` === savedPermissionedAs)
	)

	let creatingOpen = $state(false)
	function setCreating(open: boolean) {
		creatingOpen = open
		onCreatingChange?.(open)
	}
</script>

{#if suggestsAccount}
	<div class="flex flex-col items-start gap-1.5 text-2xs text-secondary">
		<span>
			It runs with your access: if you leave or lose access to <code>f/{itemFolder}</code>, it stops
			working for everyone who {kind === 'app' ? 'opens' : 'runs'} it.
		</span>
		{#if canPick && folderAccount}
			<Button
				variant="default"
				unifiedSize="xs"
				startIcon={{ icon: Bot }}
				onClick={() => onUseAccount(folderAccount.username, folderAccount.email)}
			>
				Use u/{folderAccount.username}, which reads f/{itemFolder}
			</Button>
		{:else}
			<Button
				variant="default"
				unifiedSize="xs"
				startIcon={{ icon: Bot }}
				onClick={() => setCreating(true)}
			>
				Use a service account instead
			</Button>
		{/if}
	</div>
{/if}

{#if pending}
	<div class="flex flex-col gap-1.5 rounded-md border bg-surface-secondary px-3 py-2 text-xs">
		<span class="font-semibold text-emphasis">Deploying also</span>
		<span class="flex items-start gap-2">
			<Bot size={14} class="mt-0.5 shrink-0 text-secondary" />
			<span>Creates the service account <strong>{pending.username}</strong>, half a seat</span>
		</span>
		{#if pending.grants.length}
			<span class="flex items-start gap-2">
				<Folder size={14} class="mt-0.5 shrink-0 text-secondary" />
				<span>
					Shares {pending.grants
						.map((g) => (g.kind === 'folder' ? `f/${g.path}` : g.path))
						.join(', ')} with it, read-only
				</span>
			</span>
		{/if}
		{#if pending.deployers.length}
			<span class="flex items-start gap-2">
				<Users size={14} class="mt-0.5 shrink-0 text-secondary" />
				<span>
					Lets {pending.deployers
						.map((d) => (d.startsWith('f/') ? `writers of ${d}` : d))
						.join(', ')} deploy as it
				</span>
			</span>
		{/if}
		{#if pending.defaultFor}
			<span class="flex items-start gap-2">
				<Folder size={14} class="mt-0.5 shrink-0 text-secondary" />
				<span>Makes it the default user of new items in f/{pending.defaultFor}</span>
			</span>
		{/if}
	</div>
{/if}

{#if dropsAccount}
	<Alert type="warning" size="xs" title={`This takes ${savedPermissionedAs} off the ${kind}`}>
		Only the people on {savedPermissionedAs}'s deploy list, workspace admins and members of
		{WM_DEPLOYERS_GROUP} can deploy as it. The {kind} will run with your access, and stop working for
		everyone if you leave.
	</Alert>
{/if}

<Portal>
	<Modal
		title="New service account"
		bind:open={() => creatingOpen, setCreating}
		kind="X"
		minZIndex={10001}
	>
		{#if creatingOpen}
			<div class="w-[32rem] max-w-full">
				<NewServiceAccountForm
					workspace={targetWorkspace}
					itemKind={kind}
					{itemPath}
					{needsSource}
					onBack={() => setCreating(false)}
					onUse={(account) => {
						const username = account.slice(2)
						onUseAccount(username, `${username}@${PENDING_EMAIL_DOMAIN}`)
						setCreating(false)
					}}
				/>
			</div>
		{/if}
	</Modal>
</Portal>
