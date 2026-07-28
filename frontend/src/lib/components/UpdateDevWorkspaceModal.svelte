<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert } from '$lib/components/common'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { updateDevWorkspaceModal } from '$lib/utils/editInForkModal.svelte'
	import { devWorkspaceEditUrl } from '$lib/utils/editInFork'
	import {
		checkDeployPermission,
		checkItemExists,
		deployItem,
		getOnBehalfOf,
		type DeployPermission,
		type DeployResult
	} from '$lib/utils_workspace_deploy'
	import { COMPARE_ITEMS_PARAM } from '$lib/components/sessions/modifiedItemsMask'
	import OnBehalfOfSelector, {
		needsOnBehalfOfSelection,
		type OnBehalfOfChoice,
		type OnBehalfOfDetails
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { UserService } from '$lib/gen'
	import type { Kind } from '$lib/utils_deployable'
	import { resource } from 'runed'

	const pending = $derived(updateDevWorkspaceModal.val)

	let updating = $state(false)

	// Re-checked per opened item: the modal is mounted for the whole session and the
	// rules can change under it.
	const permissionRes = resource(
		() => pending?.devWorkspaceId,
		async (dev): Promise<DeployPermission | undefined> =>
			dev ? await checkDeployPermission(dev) : undefined
	)
	const permission = $derived(permissionRes.current)

	// The compare page's update direction (prod -> dev) with this one item preselected. Where the
	// confirm button leads when the user can't deploy here, so the request still has somewhere to go.
	const compareHref = $derived(
		pending
			? `${base}/forks/compare?workspace_id=${encodeURIComponent(pending.devWorkspaceId)}` +
					`&mode=fork&direction=update` +
					`&${COMPARE_ITEMS_PARAM}=${encodeURIComponent(`${pending.itemType}:${pending.itemPath}`)}`
			: ''
	)

	// Falls open while the check is in flight — the server enforces on the deploy anyway.
	const canDeploy = $derived(permission?.ok !== false)

	// Identity the item will run under once it lands in the dev workspace. Offered only when the
	// prod item has an on_behalf_of of its own — otherwise there is no identity to carry over and
	// the deploying user is the only sensible answer (`needsOnBehalfOfSelection`).
	const sourceOnBehalfOfRes = resource(
		() => pending,
		async (req): Promise<string | undefined> =>
			req ? await getOnBehalfOf(req.itemType as Kind, req.itemPath, req.prodWorkspaceId) : undefined
	)
	const showOnBehalfOf = $derived(
		!!pending && needsOnBehalfOfSelection(pending.itemType, sourceOnBehalfOfRes.current)
	)

	// Picking anyone but yourself is admin/wm_deployers-only, mirroring the compare page's gate.
	const canPreserveRes = resource(
		() => pending?.devWorkspaceId,
		async (dev): Promise<boolean> => {
			if (!dev) return false
			try {
				const me = await UserService.whoami({ workspace: dev })
				return me.is_admin || me.groups?.includes('wm_deployers') || false
			} catch {
				return false
			}
		}
	)

	// Left unset until the user picks, and confirming is blocked meanwhile — the compare page
	// gates its deploy the same way. The selector's own "preserve the target's value" default
	// can't apply here: the item is absent from the dev workspace, which is why this prompt is up.
	let onBehalfOfChoice = $state<OnBehalfOfChoice>(undefined)
	let customOnBehalfOf = $state<OnBehalfOfDetails | undefined>(undefined)

	$effect(() => {
		pending
		onBehalfOfChoice = undefined
		customOnBehalfOf = undefined
	})

	const onBehalfOfUnset = $derived(showOnBehalfOf && onBehalfOfChoice === undefined)
	// Also blocked while the lookup is in flight: until it lands we don't know whether a choice is
	// required, and confirming immediately (Enter is bound to it) would skip one that was.
	const onBehalfOfBlocking = $derived(sourceOnBehalfOfRes.loading || onBehalfOfUnset)

	function close() {
		updateDevWorkspaceModal.val = undefined
	}

	/**
	 * Deploying `f/<folder>/<name>` into a workspace that has no `<folder>` succeeds but leaves the
	 * item orphaned — it lands with no folder to carry its permissions. The compare page avoids this
	 * because its diff lists the folder as its own item and deploys `folder:` entries first; a
	 * single-item update has to bring the folder itself. Anything else the item needs (resources,
	 * variables, resource types) is still the compare page's job, exactly as it is there.
	 */
	async function ensureFolder(req: NonNullable<typeof pending>): Promise<DeployResult> {
		const folder = req.itemPath.match(/^f\/([^/]+)\//)?.[1]
		if (!folder) return { success: true }
		const folderPath = `f/${folder}`
		try {
			if (await checkItemExists('folder', folderPath, req.devWorkspaceId)) return { success: true }
		} catch {
			// Inconclusive: let the item deploy decide rather than blocking on the probe.
			return { success: true }
		}
		return await deployItem({
			kind: 'folder',
			path: folderPath,
			workspaceFrom: req.prodWorkspaceId,
			workspaceTo: req.devWorkspaceId
		})
	}

	async function confirm() {
		const req = pending
		if (!req) return
		if (!canDeploy) {
			close()
			await goto(compareHref)
			return
		}
		updating = true
		// Folders carry no on_behalf_of, so only the item itself takes one.
		let result = await ensureFolder(req)
		if (result.success) {
			result = await deployItem({
				kind: req.itemType,
				path: req.itemPath,
				workspaceFrom: req.prodWorkspaceId,
				workspaceTo: req.devWorkspaceId,
				// Omitted for 'me' — the backend then falls back to the deploying user.
				onBehalfOf: onBehalfOfChoice === 'custom' ? customOnBehalfOf?.email : undefined
			})
		}
		updating = false
		if (!result.success) {
			// Kept open so the failure is attached to the item it happened on — a lone item can
			// fail to stand on its own (missing resource, resource type...).
			sendUserToast(
				`Could not update ${req.devWorkspaceName} with ${req.itemPath}: ${result.error}`,
				true
			)
			return
		}
		close()
		await goto(devWorkspaceEditUrl(req.itemType, req.itemPath, req.devWorkspaceId))
	}
</script>

<ConfirmationModal
	open={!!pending}
	type="info"
	title="{pending?.devWorkspaceName} is behind on this item"
	confirmationText={canDeploy ? 'Update and edit' : 'Open compare page'}
	loading={updating}
	confirmDisabled={canDeploy && onBehalfOfBlocking}
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
			{#if showOnBehalfOf}
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
						canPreserve={canPreserveRes.current ?? false}
						customValue={customOnBehalfOf?.permissionedAs}
						aboveConfirmationModal
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
