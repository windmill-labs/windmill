<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert } from '$lib/components/common'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { updateDevWorkspaceModal } from '$lib/utils/editInForkModal.svelte'
	import { claimTab, devWorkspaceEditUrl } from '$lib/utils/editInFork'
	import {
		checkItemDeployAccess,
		checkItemExists,
		createFolderIfAbsent,
		deployItem,
		getOnBehalfOfOrThrow,
		type DeployResult,
		type DeployTargetAccess
	} from '$lib/utils_workspace_deploy'
	import { COMPARE_ITEMS_PARAM } from '$lib/components/sessions/modifiedItemsMask'
	import OnBehalfOfSelector, {
		needsOnBehalfOfSelection,
		type OnBehalfOfChoice,
		type OnBehalfOfDetails
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	const pending = $derived(updateDevWorkspaceModal.val)

	let updating = $state(false)
	/** The user picker stacks above this dialog and owns the keyboard while it's up (`keyListen`). */
	let pickerOpen = $state(false)

	/**
	 * A lookup tagged with the request it answers. Requests outlive the prompt that started them
	 * (no abort signal on the client), so cancelling and reopening leaves two in flight — without
	 * the tag the last to settle decides this item's permissions and identity. `undefined` means
	 * "not looked up yet", which is what gates confirming.
	 */
	type Tagged<T> = { req: NonNullable<typeof pending>; value: T }
	function forPending<T>(res: Tagged<T> | undefined): Tagged<T> | undefined {
		return res && res.req === pending ? res : undefined
	}

	// Re-run per opened item: the modal is mounted for the whole session, the rules can change under
	// it, and write access is per-path so it can differ between two items in the same workspace.
	let accessLookup = $state<Tagged<DeployTargetAccess> | undefined>(undefined)
	let sourceLookup = $state<Tagged<{ onBehalfOf?: string; failed?: boolean }> | undefined>(
		undefined
	)

	$effect(() => {
		const req = pending
		accessLookup = undefined
		sourceLookup = undefined
		onBehalfOfChoice = undefined
		customOnBehalfOf = undefined
		if (!req) return
		let live = true
		void checkItemDeployAccess(req.devWorkspaceId, req.itemPath).then((value) => {
			if (live) accessLookup = { req, value }
		})
		// `failed` rather than `undefined`: an unreadable source is not one with no identity, and
		// conflating them would quietly hand the copy to the deploying user.
		void getOnBehalfOfOrThrow(req.itemType, req.itemPath, req.prodWorkspaceId).then(
			(onBehalfOf) => {
				if (live) sourceLookup = { req, value: { onBehalfOf } }
			},
			() => {
				if (live) sourceLookup = { req, value: { failed: true } }
			}
		)
		return () => {
			live = false
		}
	})

	const access = $derived(forPending(accessLookup))
	const permission = $derived(access?.value.permission)

	// The compare page's update direction (prod -> dev) with this one item preselected. Where the
	// confirm button leads when the user can't deploy here, so the request still has somewhere to go.
	const compareHref = $derived(
		pending
			? `${base}/forks/compare?workspace_id=${encodeURIComponent(pending.devWorkspaceId)}` +
					`&mode=fork&dir=update` +
					`&${COMPARE_ITEMS_PARAM}=${encodeURIComponent(`${pending.itemType}:${pending.itemPath}`)}`
			: ''
	)

	// Falls open while the check is in flight so the modal doesn't flash a refusal it may retract;
	// confirming stays blocked until it lands (see `confirmBlocked`).
	const canDeploy = $derived(permission?.ok !== false)

	// Identity the item will run under once it lands in the dev workspace. Offered only when the
	// prod item has an on_behalf_of of its own — otherwise there is no identity to carry over and
	// the deploying user is the only sensible answer (`needsOnBehalfOfSelection`).
	const sourceOnBehalfOf = $derived(forPending(sourceLookup))
	const showOnBehalfOf = $derived(
		!!pending && needsOnBehalfOfSelection(pending.itemType, sourceOnBehalfOf?.value.onBehalfOf)
	)

	// Left unset until the user picks, and confirming is blocked meanwhile. The selector's own
	// "preserve the target's value" default can't apply: the item is absent from the dev workspace.
	let onBehalfOfChoice = $state<OnBehalfOfChoice>(undefined)
	let customOnBehalfOf = $state<OnBehalfOfDetails | undefined>(undefined)

	// 'me' is sent explicitly rather than left blank. Sending nothing means "no preference", which
	// lets the target folder's `default_permissioned_as` claim the item — so the option labelled
	// "me" would deploy it as somebody else. No choice at all (selector hidden) still defers to it.
	const chosenIdentity = $derived(
		onBehalfOfChoice === 'custom'
			? customOnBehalfOf
			: onBehalfOfChoice === 'me'
				? access?.value.me
				: undefined
	)

	const onBehalfOfUnset = $derived(showOnBehalfOf && onBehalfOfChoice === undefined)
	// Blocked until both lookups land *for this item* — Enter is bound to confirm, so a fast one
	// would otherwise deploy past the permission check and skip a required choice. Not blocked once
	// refused: the button leads to the compare page then.
	const sourceOnBehalfOfFailed = $derived(!!sourceOnBehalfOf?.value.failed)
	// An identity has to be picked but we don't know who "me" is there, so no choice can be honoured:
	// sending nothing would hand the item to the folder default instead.
	const targetIdentityUnknown = $derived(showOnBehalfOf && !!access && !access.value.me)
	const confirmBlocked = $derived(
		canDeploy &&
			(!access ||
				!sourceOnBehalfOf ||
				sourceOnBehalfOfFailed ||
				targetIdentityUnknown ||
				onBehalfOfUnset)
	)

	function close() {
		updateDevWorkspaceModal.val = undefined
	}

	/**
	 * Deploying `f/<folder>/<name>` into a workspace with no `<folder>` succeeds but orphans the
	 * item — it lands with no folder to carry its permissions. Anything else it needs (resources,
	 * variables, resource types) stays the compare page's job, exactly as it is there.
	 */
	async function ensureFolder(req: NonNullable<typeof pending>): Promise<DeployResult> {
		const folder = req.itemPath.match(/^f\/([^/]+)\//)?.[1]
		if (!folder) return { success: true }
		const folderPath = `f/${folder}`
		try {
			if (await checkItemExists('folder', folderPath, req.devWorkspaceId)) return { success: true }
		} catch (e) {
			// The one probe that must not fail open: deploying while the folder is in fact missing is
			// the orphaning above, and nothing downstream would catch it.
			return { success: false, error: `could not check whether ${folderPath} exists (${e})` }
		}
		// Create-only: nobody asked for this folder to be deployed, so it must never overwrite one
		// that appeared meanwhile. See `createFolderIfAbsent`.
		const result = await createFolderIfAbsent(folder, req.prodWorkspaceId, req.devWorkspaceId)
		if (result.droppedAccess?.length) {
			// Narrower than the source rather than wider, so it doesn't block the deploy — but it is
			// still not what the folder looked like where it came from.
			sendUserToast(
				`${folderPath} was created without access for ${result.droppedAccess.join(', ')} — ` +
					`no such user or group in ${req.devWorkspaceName}`
			)
		}
		return result
	}

	async function presenceInDev(
		req: NonNullable<typeof pending>
	): Promise<'present' | 'absent' | 'unknown'> {
		try {
			return (await checkItemExists(req.itemType, req.itemPath, req.devWorkspaceId))
				? 'present'
				: 'absent'
		} catch {
			return 'unknown'
		}
	}

	async function confirm() {
		const req = pending
		if (!req || updating) return
		// Claimed before the first `await`, for the same reason the dropdown entry claims one: a tab
		// opened from a promise continuation never appears on Safari. Released on every path that
		// leaves the prompt up, so a retry starts from a clean slate.
		const tab = req.openInNewTab ? claimTab() : undefined
		async function leaveTo(url: string, destination: string) {
			if (tab) tab.show(url)
			else if (req!.openInNewTab) {
				// The claim was blocked, and so is this. Every caller has already closed the prompt and
				// may have deployed, so staying silent would read as the confirm having done nothing.
				if (!window.open(url)) sendUserToast(`Allow popups to open ${destination}`, true)
			} else await goto(url)
		}
		const itemInDev = `${req.itemPath} in ${req.devWorkspaceName}`
		if (!canDeploy) {
			// Read before closing: the href is derived from the request being answered, so clearing it
			// first leaves nothing to navigate to.
			const href = compareHref
			close()
			await leaveTo(href, 'the compare page')
			return
		}
		updating = true
		// Folders carry no on_behalf_of, so only the item itself takes one.
		let result = await ensureFolder(req)
		if (result.success) {
			// The prompt is only up because the item was absent, so this asks once more before writing
			// and the write itself refuses to become an update (`createOnly` below). Between them,
			// whoever landed it meanwhile is opened rather than overwritten.
			const presence = await presenceInDev(req)
			if (presence === 'present') {
				updating = false
				close()
				sendUserToast(`${req.itemPath} is already in ${req.devWorkspaceName}, opening it`)
				await leaveTo(
					devWorkspaceEditUrl(req.itemType, req.itemPath, req.devWorkspaceId),
					itemInDev
				)
				return
			}
			if (presence === 'unknown') {
				// Someone may have landed it meanwhile and writing would overwrite them, so this probe
				// can't fail open either. Prompt stays up so a retry is one click away.
				updating = false
				tab?.discard()
				sendUserToast(
					`Could not check whether ${req.itemPath} is already in ${req.devWorkspaceName}`,
					true
				)
				return
			}
			const deployed = await deployItem({
				kind: req.itemType,
				path: req.itemPath,
				workspaceFrom: req.prodWorkspaceId,
				workspaceTo: req.devWorkspaceId,
				onBehalfOf: chosenIdentity?.email,
				onBehalfOfPrincipal: chosenIdentity?.permissionedAs,
				createOnly: true
			})
			if (deployed.conflict) {
				// Landed between the probe above and the write, and `createOnly` refused rather than
				// replacing it. Their version stands; open it, as the probe's own branch does.
				updating = false
				close()
				sendUserToast(`${req.itemPath} is already in ${req.devWorkspaceName}, opening it`)
				await leaveTo(
					devWorkspaceEditUrl(req.itemType, req.itemPath, req.devWorkspaceId),
					itemInDev
				)
				return
			}
			result = deployed
		}
		updating = false
		if (!result.success) {
			// Kept open so the failure is attached to the item it happened on — a lone item can
			// fail to stand on its own (missing resource, resource type...).
			tab?.discard()
			sendUserToast(
				`Could not update ${req.devWorkspaceName} with ${req.itemPath}: ${result.error}`,
				true
			)
			return
		}
		close()
		await leaveTo(devWorkspaceEditUrl(req.itemType, req.itemPath, req.devWorkspaceId), itemInDev)
	}
</script>

<ConfirmationModal
	open={!!pending}
	type="info"
	title="{pending?.devWorkspaceName} is behind on this item"
	confirmationText={canDeploy ? 'Update and edit' : 'Open compare page'}
	loading={updating}
	keyListen={!pickerOpen}
	confirmDisabled={confirmBlocked}
	onConfirmed={confirm}
	onCanceled={close}
>
	{#if pending}
		<p>
			<span class="font-mono">{pending.itemPath}</span>
			exists in <b>{pending.prodWorkspaceId}</b> but not in its dev workspace
			<b>{pending.devWorkspaceName}</b>.
		</p>
		{#if canDeploy}
			<p class="mt-2">
				Update <b>{pending.devWorkspaceName}</b> with it to edit it there.
			</p>
			{#if sourceOnBehalfOfFailed}
				<div class="mt-2">
					<Alert type="error" size="xs" title="Could not read {pending.itemPath}">
						Its "run on behalf of" user is unknown, so updating could silently reassign the item to
						you. Retry from the compare page.
					</Alert>
				</div>
			{:else if targetIdentityUnknown}
				<div class="mt-2">
					<Alert
						type="error"
						size="xs"
						title="Could not read your account in {pending.devWorkspaceName}"
					>
						This item needs a "run on behalf of" user and none can be applied without it. Retry from
						the compare page.
					</Alert>
				</div>
			{:else if showOnBehalfOf}
				<div class="mt-3 flex items-center gap-2">
					<span class="text-xs text-secondary">Runs on behalf of</span>
					<OnBehalfOfSelector
						targetWorkspace={pending.devWorkspaceId}
						targetValue={undefined}
						selected={onBehalfOfChoice}
						onSelect={(choice, details) => {
							onBehalfOfChoice = choice
							if (details) customOnBehalfOf = details
						}}
						kind={pending.itemType}
						canPreserve={access?.value.canPreserveOnBehalfOf ?? false}
						customValue={customOnBehalfOf?.permissionedAs}
						aboveConfirmationModal
						onPickerOpenChange={(open) => (pickerOpen = open)}
						myPermissionedAs={access?.value.me?.permissionedAs}
					/>
				</div>
				{#if onBehalfOfUnset}
					<span class="text-xs text-yellow-600">
						You must set the "on behalf of" user before updating
						<Tooltip class="text-yellow-600">
							The "run on behalf of" field defines which user's permissions will be applied during
							execution. Make sure this is set to an appropriate user before updating.
						</Tooltip>
					</span>
				{/if}
			{/if}
		{:else if permission}
			<div class="mt-2">
				<Alert type="warning" size="xs" title="You can't update {pending.devWorkspaceName}">
					{permission.reason}
				</Alert>
			</div>
		{/if}
	{/if}
</ConfirmationModal>
