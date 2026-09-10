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
 * Whether the workspace can store uploaded files. Call during component initialisation and
 * read `.current` where the answer is used.
 *
 * `whileLoading` is what to answer before this workspace's own settings land, and the two
 * kinds of caller want opposites: a field warning "no storage configured" must not flash
 * on every navigation, while a control that uploads must not be offered before it is known
 * to work.
 */
export function useWorkspaceStorageConfigured(
	ws: () => string | undefined,
	whileLoading: boolean
): {
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
			: whileLoading
	})

	return {
		get current() {
			return configured
		}
	}
}

/**
 * The same answer for the fields that warn without storage: assumed configured until this
 * workspace's own answer lands, so the warning never lingers from the previous workspace
 * nor appears merely because the fetch failed.
 */
export function useS3StorageConfigured(ws: () => string | undefined): {
	readonly current: boolean
} {
	return useWorkspaceStorageConfigured(ws, true)
}
