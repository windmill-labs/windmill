<script lang="ts">
	import { Share2 } from 'lucide-svelte'
	import { fromStore } from 'svelte/store'
	import { Button } from '$lib/components/common'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { sendUserToast } from '$lib/toast'
	import { apiErrorMessage } from '$lib/utils'
	import type { PersistedArtifact } from './artifactsDB'
	import { homeWorkspace } from './artifactLibrary'
	import ArtifactShareDialog from './ArtifactShareDialog.svelte'

	interface Props {
		artifactId: string
		name: string
		/** The owner's copy in this browser, which a share puts on the server first. */
		artifact?: PersistedArtifact
		/** The server copy's path, for an artifact this browser does not hold. */
		serverPath?: string
		/** The artifact's home workspace, when the caller knows it. */
		workspace?: string
		disabled?: boolean
		/** Called after the access list or the version people see changed. */
		onChange?: () => void
	}

	let {
		artifactId,
		name,
		artifact,
		serverPath,
		workspace,
		disabled = false,
		onChange
	}: Props = $props()

	const operating = fromStore(useOperatingWorkspace())
	const home = $derived(
		workspace ??
			artifact?.workspaceId ??
			(operating.current ? homeWorkspace(operating.current) : undefined)
	)

	let dialog: ArtifactShareDialog | undefined = $state()
	let opening = $state(false)

	async function share() {
		opening = true
		try {
			await dialog?.share({ id: artifactId, name, local: artifact, path: serverPath })
		} catch (err) {
			sendUserToast('Could not share the artifact', true, [], apiErrorMessage(err))
		} finally {
			opening = false
		}
	}
</script>

<Button
	unifiedSize="sm"
	variant="default"
	startIcon={{ icon: Share2 }}
	loading={opening}
	disabled={disabled || !home || (!artifact && !serverPath)}
	onclick={share}
>
	Share
</Button>
<ArtifactShareDialog bind:this={dialog} workspace={home} {onChange} />
