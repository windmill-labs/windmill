<script lang="ts">
	import ShareDialog from '$lib/components/itemSettings/ShareDialog.svelte'
	import {
		setOperatingWorkspace,
		setReadOnlyWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import type { PersistedArtifact } from './artifactsDB'
	import { ArtifactWriter } from './artifactWriter'
	import { httpArtifactServer } from './httpArtifactServer'

	interface Props {
		/** The artifacts' home workspace, where their server copies are. */
		workspace: string | undefined
		/** Called after an access list or the version people see changed. */
		onChange?: () => void
	}

	let { workspace, onChange }: Props = $props()

	setOperatingWorkspace(() => workspace)
	// A session in Use mode changes nothing in its workspace, but sharing what it wrote is the
	// point of many of them: the access list stays the owner's to edit.
	setReadOnlyWorkspace(() => false)

	let dialog: ShareDialog | undefined = $state()
	const writer = new ArtifactWriter(httpArtifactServer)

	/** Open the share dialog for an artifact, putting this browser's copy on the server first:
	 * the access list belongs to the server copy. Rejects when that copy could not be made. */
	export async function share(a: {
		id: string
		name: string
		local?: PersistedArtifact
		/** The server copy's path, for an artifact this browser does not hold. */
		path?: string
	}): Promise<void> {
		if (!workspace) return
		const path = a.local ? ((await writer.publish(a.id, workspace))?.path ?? a.path) : a.path
		if (!path) return
		dialog?.openDialog('ai_artifact', path, { source: 'artifact', artifactId: a.id, title: a.name })
	}
</script>

<ShareDialog bind:this={dialog} {onChange} />
