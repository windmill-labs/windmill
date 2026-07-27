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
		deployItem,
		type DeployPermission
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

	async function confirm() {
		const req = pending
		if (!req) return
		if (!canDeploy) {
			close()
			await goto(compareHref)
			return
		}
		copying = true
		const result = await deployItem({
			kind: req.itemType,
			path: req.itemPath,
			workspaceFrom: req.prodWorkspaceId,
			workspaceTo: req.devWorkspaceId
		})
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
