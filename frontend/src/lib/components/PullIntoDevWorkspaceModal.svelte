<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Alert } from '$lib/components/common'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { pullIntoDevModal } from '$lib/utils/editInForkModal.svelte'
	import { devWorkspaceItemUrl } from '$lib/utils/editInFork'
	import {
		checkDeployPermission,
		checkItemExists,
		deployItem,
		type DeployPermission,
		type DeployResult
	} from '$lib/utils_workspace_deploy'
	import { COMPARE_ITEMS_PARAM } from '$lib/components/sessions/modifiedItemsMask'
	import { resource } from 'runed'

	const pending = $derived(pullIntoDevModal.val)

	let copying = $state(false)

	// Re-checked per opened item: the modal is mounted for the whole session and the
	// rules can change under it.
	const permissionRes = resource(
		() => pending?.devWorkspaceId,
		async (dev): Promise<DeployPermission | undefined> =>
			dev ? await checkDeployPermission(dev) : undefined
	)
	const permission = $derived(permissionRes.current)

	// The compare page's update direction (prod -> dev) with this one item preselected,
	// so "the copy needs more than this item" stays one click away.
	const compareHref = $derived(
		pending
			? `${base}/forks/compare?workspace_id=${encodeURIComponent(pending.devWorkspaceId)}` +
					`&mode=fork&direction=update` +
					`&${COMPARE_ITEMS_PARAM}=${encodeURIComponent(`${pending.itemType}:${pending.itemPath}`)}`
			: ''
	)

	// Falls open while the check is in flight — the server enforces on the deploy anyway.
	const canDeploy = $derived(permission?.ok !== false)

	function close() {
		pullIntoDevModal.val = undefined
	}

	/**
	 * Deploying `f/<folder>/<name>` into a workspace that has no `<folder>` succeeds but leaves the
	 * item orphaned — it lands with no folder to carry its permissions. The compare page avoids this
	 * because its diff lists the folder as its own item and deploys `folder:` entries first; a
	 * single-item copy has to pull the folder itself. Anything else the item needs (resources,
	 * variables, resource types) is still the compare page's job, exactly as it is for an update.
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
		copying = true
		// No `onBehalfOf`: the compare page's default is the deploying user's identity, and its other
		// choices read the value the item already has in the target — which by definition it hasn't here.
		let result = await ensureFolder(req)
		if (result.success) {
			result = await deployItem({
				kind: req.itemType,
				path: req.itemPath,
				workspaceFrom: req.prodWorkspaceId,
				workspaceTo: req.devWorkspaceId
			})
		}
		copying = false
		if (!result.success) {
			// Kept open: the compare-page link below is the way out when a lone item
			// can't stand on its own (missing folder, resource, resource type...).
			sendUserToast(
				`Could not copy ${req.itemPath} to ${req.devWorkspaceName}: ${result.error}`,
				true
			)
			return
		}
		close()
		await goto(devWorkspaceItemUrl(req.itemType, req.itemPath, req.devWorkspaceId))
	}
</script>

<ConfirmationModal
	open={!!pending}
	type="info"
	title="Not in {pending?.devWorkspaceName} yet"
	confirmationText={canDeploy ? 'Copy and edit' : 'Open compare page'}
	loading={copying}
	onConfirmed={confirm}
	onCanceled={close}
>
	{#if pending}
		<p>
			<span class="font-mono">{pending.itemPath}</span>
			exists in <b>{pending.prodWorkspaceId}</b> but not in its dev workspace
			<b>{pending.devWorkspaceName}</b>, which is behind on it.
		</p>
		{#if canDeploy}
			<p class="mt-2">
				Copy it over to edit it there, or
				<a href={compareHref} onclick={close}>review it on the compare page</a>
				to pull it alongside everything else this workspace is behind on.
			</p>
		{:else if permission}
			<div class="mt-2">
				<Alert type="warning" size="xs" title="You can't deploy into {pending.devWorkspaceName}">
					{permission.reason}
				</Alert>
			</div>
		{/if}
	{/if}
</ConfirmationModal>
