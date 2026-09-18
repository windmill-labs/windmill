import { CancelError, WorkspaceService, type LargeFileStorage } from '$lib/gen'
import { resource } from 'runed'

/**
 * Whether the workspace has large-file storage the upload endpoints can resolve. Every
 * kind counts, not only S3: Azure Blob, Azure Workload Identity, S3 via AWS OIDC and GCS
 * all go through the same object-store abstraction, so reading `s3_resource_path` alone
 * calls a perfectly good workspace unconfigured.
 */
function storageConfigured(storage: LargeFileStorage | undefined): boolean {
	if (!storage) return false
	return (
		storage.type !== undefined ||
		storage.s3_resource_path !== undefined ||
		storage.azure_blob_resource_path !== undefined ||
		storage.gcs_resource_path !== undefined
	)
}

/**
 * Whether the workspace can store uploaded files; read `.current`. Assumed configured until
 * this workspace's own answer lands, so "no storage" never flashes on navigation or shows
 * merely because the fetch failed.
 */
export function useWorkspaceStorageConfigured(ws: () => string | undefined): {
	readonly current: boolean
} {
	const settings = resource(ws, async (ws, _previousWs, { onCleanup }) => {
		if (!ws) return undefined
		const req = WorkspaceService.getPublicSettings({ workspace: ws })
		// `resource` keeps whatever lands last: cancel a superseded request so a slow
		// reply for a workspace we have left cannot overwrite the current one.
		onCleanup(() => req.cancel())
		try {
			return { ws, settings: await req }
		} catch (err) {
			if (!(err instanceof CancelError)) {
				console.error('Failed to fetch workspace settings:', err)
			}
			return undefined
		}
	})

	const configured = $derived.by(() => {
		const loaded = settings.current
		return loaded && loaded.ws === ws()
			? storageConfigured(loaded.settings.large_file_storage)
			: true
	})

	return {
		get current() {
			return configured
		}
	}
}
