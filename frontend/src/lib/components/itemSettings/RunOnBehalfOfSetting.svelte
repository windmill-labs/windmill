<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { userStore } from '$lib/stores'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import SettingToggle from './SettingToggle.svelte'

	const WM_DEPLOYERS_GROUP = 'wm_deployers'
	const DOCS_LINK =
		'https://www.windmill.dev/docs/core_concepts/roles_and_permissions#run-on-behalf-of'

	interface Props {
		kind: 'flow' | 'script' | 'app'
		/** Email of the user whose access the runs use; undefined uses whoever starts each run. */
		email: string | undefined
		/** `u/username` of that user. */
		permissionedAs: string | undefined
		/** Sent as `preserve_on_behalf_of` on deploy. Undefined until a user is picked here, which
		 *  the host reads as "keep the deployed identity". */
		preserve: boolean | undefined
		/** The deployed identity, which the "target" choice keeps. */
		savedEmail: string | undefined
		savedPermissionedAs: string | undefined
		targetWorkspace: string
		/** Apps always use a chosen user's access: no toggle, only the user choice. */
		required?: boolean
		/** Never deployed, so no deployed identity can change. */
		isNew?: boolean
		tooltip?: string
	}

	let {
		kind,
		email = $bindable(),
		permissionedAs = $bindable(),
		preserve = $bindable(),
		savedEmail,
		savedPermissionedAs,
		targetWorkspace,
		required = false,
		isNew = false,
		tooltip
	}: Props = $props()

	// The copy names the access runs get, not the identity the backend records them under: the
	// person who starts a run stays recorded on it and readable from WM_END_USER_EMAIL, so wording
	// about who "runs" it misleads.
	let shownTooltip = $derived(
		tooltip ??
			`For example, a ${kind} reads the finance database. Give it the access of finance-bot, and support agents can run it without seeing the database credentials. The docs call this “run on behalf of”.`
	)

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)
	let canPreserve = $derived(
		!!actingUser?.is_admin ||
			!!actingUser?.is_super_admin ||
			(actingUser?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
	)
	// Usernames are per-workspace, so "me" is the acting user in the target workspace.
	let myPermissionedAs = $derived(actingUser?.username ? `u/${actingUser.username}` : undefined)

	let choice: OnBehalfOfChoice = $state(undefined)
	let customEmail = $state('')

	let enabled = $derived(required || !!email)
	let chosenUser = $derived(permissionedAs ?? email ?? 'the chosen user')
	// Without the right to preserve, the backend runs the redeployed item as the deployer whatever
	// is submitted, as it does for "me".
	let deployPermissionedAs = $derived.by(() => {
		if (!enabled) return undefined
		if (!canPreserve || choice === 'me') return myPermissionedAs
		if (choice === 'custom') return permissionedAs
		return savedPermissionedAs
	})
	let willChange = $derived(
		!isNew &&
			!!savedPermissionedAs &&
			!!deployPermissionedAs &&
			deployPermissionedAs !== savedPermissionedAs
	)

	function setMe() {
		email = $userStore?.email
		permissionedAs = myPermissionedAs
		customEmail = ''
		preserve = false
	}
</script>

{#snippet picker()}
	<div class="flex flex-wrap items-center gap-1.5">
		<OnBehalfOfSelector
			{targetWorkspace}
			targetValue={savedEmail}
			selected={choice ?? (savedEmail ? undefined : 'me')}
			onSelect={(c, details) => {
				choice = c
				if (c === 'me') {
					setMe()
				} else if (c === 'target') {
					// Keep the saved pair. An item that has no recorded principal yet sends the
					// email alone and the backend derives one from it.
					email = savedEmail
					permissionedAs = savedPermissionedAs
					customEmail = ''
					preserve = true
				} else if (c === 'custom' && details) {
					email = details.email
					permissionedAs = details.permissionedAs
					customEmail = details.email
					preserve = true
				}
			}}
			{kind}
			{canPreserve}
			customValue={customEmail}
			isDeployment={false}
			{myPermissionedAs}
			readOnlyWithoutPreserve
		/>
		{#if willChange}
			<AlertTriangle size={14} class="text-yellow-500" />
			<span class="text-2xs text-yellow-600 dark:text-yellow-400"
				>Changes from <strong>{savedPermissionedAs}</strong> on deploy</span
			>
		{/if}
	</div>
	{#if !canPreserve}
		<div class="text-2xs text-secondary">
			Only workspace admins and members of {WM_DEPLOYERS_GROUP} can pick another user.
		</div>
	{/if}
	<div class="text-2xs text-secondary">
		{#if required}
			Viewers need no access to the app's resources and scripts. Each runnable still gets the
			viewer's email in <code>WM_END_USER_EMAIL</code>.
		{:else}
			Runs reach what {chosenUser} can access. Each run still records who started it, and the code gets
			their email in <code>WM_END_USER_EMAIL</code>.
		{/if}
	</div>
{/snippet}

{#snippet valueHint()}
	Lets people run this {kind} without access to the resources, variables and scripts it uses.
{/snippet}

{#if required}
	<div data-setting="on-behalf-of">
		<Label label="Runnables use the access of">
			{#snippet header()}
				<Tooltip documentationLink={DOCS_LINK}>{shownTooltip}</Tooltip>
			{/snippet}
			{@render picker()}
		</Label>
	</div>
{:else}
	<SettingToggle
		setting="on-behalf-of"
		label="Use a chosen user's access"
		tooltip={shownTooltip}
		documentationLink={DOCS_LINK}
		hint={valueHint}
		checked={enabled}
		onchange={(on) => {
			if (on) {
				setMe()
				choice = 'me'
			} else {
				email = undefined
				permissionedAs = undefined
				preserve = false
				choice = undefined
			}
		}}
	>
		{@render picker()}
	</SettingToggle>
{/if}
