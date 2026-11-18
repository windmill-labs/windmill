import { resource } from 'runed'
import { AiService, UserService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { readSessionSnapshot } from './sessionSharing'
import { invalidateSharedSessions } from './sharedSessionsList.svelte'
import { logSharedSessionOpen, logSharedSessionRemove } from './sharedSessionTelemetry'

export type LoadedSharedSession =
	| {
			state: 'found'
			session: Awaited<ReturnType<typeof AiService.getSharedAiSession>>
			snapshot: ReturnType<typeof readSessionSnapshot>
	  }
	| { state: 'gone' }
	| { state: 'outsider'; email: string }
	| { state: 'error'; message: string }

/** Loads a share and stops sharing it. Nothing is fetched while `key` returns undefined. */
export function useSharedSession(key: () => { workspace: string; id: string } | undefined) {
	const shared = resource(key, async (args): Promise<LoadedSharedSession | undefined> => {
		if (!args?.workspace || !args.id) return undefined
		try {
			const session = await AiService.getSharedAiSession(args)
			// Reading someone else's share is what lists it for the reader, and the sidebar may
			// have fetched its list before this read recorded it.
			if (!session.mine) {
				invalidateSharedSessions(args.workspace)
				logSharedSessionOpen(args.workspace)
			}
			return { state: 'found', session, snapshot: readSessionSnapshot(session.snapshot) }
		} catch (err) {
			const status = (err as { status?: number })?.status
			if (status === 401) {
				// The login page sends a signed-in user straight back to `rd`, so only a visitor
				// with no session goes there; anyone else is outside the share's workspace. The
				// server answers them alike whether or not the share exists, so the message must
				// not say that it does.
				const email = await UserService.getCurrentEmail().catch(() => undefined)
				if (email) return { state: 'outsider', email }
				const rd = window.location.pathname + window.location.search
				window.location.replace(`/user/login?rd=${encodeURIComponent(rd)}`)
				return undefined
			}
			// A malformed id is a 400 from the path extractor, and means the same to the reader.
			if (status === 404 || status === 400) return { state: 'gone' }
			return { state: 'error', message: String((err as { body?: unknown })?.body ?? err) }
		}
	})

	const found = $derived(shared.current?.state === 'found' ? shared.current : undefined)
	let removing = $state(false)

	async function stopSharing() {
		const workspace = key()?.workspace
		if (!found || !workspace) return
		removing = true
		try {
			await AiService.unshareAiSession({ workspace, id: found.session.id })
			shared.mutate({ state: 'gone' })
			invalidateSharedSessions(workspace)
			sendUserToast('Stopped sharing: the link no longer opens')
		} catch (err) {
			sendUserToast(`Could not stop sharing: ${(err as { body?: unknown })?.body ?? err}`, true)
		} finally {
			removing = false
		}
	}

	/** Takes the share out of the caller's sidebar; the link keeps working. */
	async function removeFromList(): Promise<boolean> {
		const workspace = key()?.workspace
		if (!found || !workspace) return false
		removing = true
		try {
			await AiService.forgetOpenedAiSession({ workspace, id: found.session.id })
			invalidateSharedSessions(workspace)
			logSharedSessionRemove('header', workspace)
			sendUserToast('Removed from your shared sessions')
			return true
		} catch (err) {
			sendUserToast(`Could not remove it: ${(err as { body?: unknown })?.body ?? err}`, true)
			return false
		} finally {
			removing = false
		}
	}

	return {
		get current() {
			return shared.current
		},
		get found() {
			return found
		},
		get removing() {
			return removing
		},
		stopSharing,
		removeFromList
	}
}
