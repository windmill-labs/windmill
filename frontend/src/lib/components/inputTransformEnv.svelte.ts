import { CancelError, WorkspaceService } from '$lib/gen'
import { resource } from 'runed'

/**
 * Whether the workspace has S3 storage configured, for the fields that warn without it. Call during
 * component initialisation and read `.current` where the answer is used.
 */
export function useS3StorageConfigured(ws: () => string | undefined): {
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

	// Assume configured until this workspace's own answer lands: the warning must not
	// linger from the previous workspace, nor appear merely because the fetch failed.
	const configured = $derived.by(() => {
		const loaded = settings.current
		return loaded && loaded.ws === ws()
			? loaded.settings.large_file_storage?.s3_resource_path !== undefined
			: true
	})

	return {
		get current() {
			return configured
		}
	}
}
