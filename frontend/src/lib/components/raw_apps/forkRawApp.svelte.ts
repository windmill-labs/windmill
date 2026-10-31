import { get } from 'svelte/store'
import type { Policy } from '$lib/gen'
import { userStore } from '$lib/stores'
import { sendUserToast } from '$lib/toast'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { mintDraftPath } from '$lib/mintDraftPath'
import { userPathPrefix } from '$lib/utils'
import { random_adj } from '$lib/components/random_positive_adjetive'
import type { RawAppDraft } from '$lib/components/sessions/appDraftCodec'
import type { RawAppData } from './dataTableRefUtils'

/**
 * Saves what the editor holds as a new raw-app draft under a fresh path and returns that
 * path, or `undefined` when the save failed. The copy is taken from the editor rather than
 * reloaded from the server, which would miss edits autosave has not written (or never will,
 * with auto-save off).
 */
export async function saveRawAppFork(
	workspace: string,
	source: {
		files: Record<string, string>
		runnables: Record<string, any>
		data: RawAppData
		policy: Policy | undefined
	}
): Promise<string | undefined> {
	const user = get(userStore)
	const forkPath = mintDraftPath()
	const draft: RawAppDraft = {
		summary: '',
		files: $state.snapshot(source.files),
		runnables: $state.snapshot(source.runnables),
		data: $state.snapshot(source.data),
		// The forker's identity, since the source's would publish as its owner, but the
		// source's sandbox and SDK scopes: the copied code's SDK calls depend on them.
		policy: {
			on_behalf_of: user?.username.includes('@') ? user.username : `u/${user?.username}`,
			on_behalf_of_email: user?.email,
			execution_mode: 'publisher',
			sandbox: source.policy?.sandbox ?? false,
			frontend_sdk_scopes: $state.snapshot(source.policy?.frontend_sdk_scopes)
		},
		// Without it the fork would deploy under its `draft_<uuid>` storage path.
		draft_path: `${userPathPrefix(user?.username)}${random_adj()}_app`
	}
	UserDraft.save('raw_app', forkPath, draft, { workspace })
	await UserDraft.forcePersist('raw_app', forkPath, { workspace })
	const saved = UserDraftDbSyncer.getState({ workspace, itemKind: 'raw_app', path: forkPath })
	if (saved.state === 'failed') {
		sendUserToast(`Could not save the fork: ${saved.failureMessage ?? 'unknown error'}`, true)
		return undefined
	}
	return forkPath
}
