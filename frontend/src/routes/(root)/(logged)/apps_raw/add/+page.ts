import { redirect } from '@sveltejs/kit'
import { get } from 'svelte/store'
import { makeDraftAddLoad } from '$lib/draftAddRedirect'
import { mintDraftPath } from '$lib/mintDraftPath'
import { userStore, workspaceStore } from '$lib/stores'
import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
import { loadCopilot } from '$lib/components/copilot/loadCopilot'
import { prefersSessionHandoff } from '$lib/components/copilot/chat/global/gate'
import { editorSessionHref } from '$lib/components/sessions/sessionSwitch.svelte'
import { importStore } from '$lib/components/apps/store'
import type { PageLoad } from './$types'

export const prerender = false

const openInEditor = makeDraftAddLoad('apps_raw/edit')

/** A blank new app is set up in an AI session, the recommended way to build one, whenever
 * the user has sessions and AI to build with. Anything that arrives with content of its
 * own (an import, a seeded path) is an editor hand-off and stays one. */
async function newAppSessionHref(url: URL): Promise<string | undefined> {
	let hasOwnParams = false
	url.searchParams.forEach((_, k) => (hasOwnParams ||= k !== 'workspace'))
	if (hasOwnParams || get(importStore) || sessionStorage.getItem('rawAppImport')) {
		return undefined
	}
	const user = get(userStore)
	const workspace = get(workspaceStore)
	// Neither is known yet on a cold load of this URL, which then opens the editor.
	if (!user || !workspace || !prefersSessionHandoff(user.operator)) return undefined
	if ((url.searchParams.get('workspace') ?? workspace) !== workspace) return undefined
	if (get(copilotWorkspace) !== workspace) await loadCopilot(workspace)
	if (get(copilotWorkspace) !== workspace || !get(copilotInfo).enabled) return undefined
	return editorSessionHref({ kind: 'raw_app', path: mintDraftPath() }, workspace, {
		new_draft: 'true'
	})
}

export const load: PageLoad = async (event) => {
	const href = await newAppSessionHref(event.url)
	if (href) redirect(307, href)
	return openInEditor(event)
}
