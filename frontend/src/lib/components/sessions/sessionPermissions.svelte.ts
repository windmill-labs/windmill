import { getContext } from 'svelte'
import { fromStore } from 'svelte/store'
import { resource } from 'runed'
import { userWorkspaces } from '$lib/stores'
import { useActingUser } from '$lib/actingUser.svelte'
import { useEditRights } from '$lib/operatorWriteRights'
import { buildsAnything } from '$lib/editRights'
import {
	canWorkDirectly,
	forgetReadOnlyVerdicts,
	isReadOnlySessionWorkspace,
	settledReadOnlyVerdict
} from '$lib/components/copilot/chat/global/sessionAccess'
import { AIWorkMode } from '$lib/components/copilot/chat/workMode'
import type { WorkspaceItem } from '$lib/components/workspacePicker'
import { previewPageAllowed, type PreviewPage } from './previewRouter'

let verdictEpoch = $state(0)

/** Every `useSessionReadOnly` reads the rules again: the sessions view stays mounted while the
 * user is on other pages, where the rules can change. */
export function rereadReadOnlyVerdicts(): void {
	forgetReadOnlyVerdicts()
	verdictEpoch++
}

/** Whether a session in `workspace` is read-only because the workspace is protected: the verdict
 * `resolveSessionAccess` gives the chat, so the UI and the toolset agree. `direct`: the session
 * was opted into working there directly, which lifts it for the rule's bypassers. */
export function useSessionReadOnly(
	workspace: () => string | undefined,
	direct: () => boolean = () => false
) {
	const verdict = resource(
		[workspace, direct, () => verdictEpoch],
		async ([ws, isDirect], _prev, { signal }) => {
			if (!ws) return undefined
			const readOnly =
				(await isReadOnlySessionWorkspace(ws)) && !(isDirect && (await canWorkDirectly(ws)))
			// A superseded read answers for older rules: runed would still land it over the newer one.
			if (signal.aborted) throw new DOMException('superseded', 'AbortError')
			// Only a successful read settles a verdict; a failed one is read-only to fail closed.
			return {
				workspace: ws,
				direct: isDirect,
				readOnly,
				rulesRead: settledReadOnlyVerdict(ws) !== undefined
			}
		}
	)
	// `resource` keeps the previous workspace's verdict while it asks about the next one. A direct
	// session waits for its own verdict rather than take the workspace's.
	const verdictFor = (ws: string): boolean | undefined => {
		const landed = verdict.current
		if (landed?.workspace === ws && landed.direct === direct()) return landed.readOnly
		return direct() ? undefined : settledReadOnlyVerdict(ws)
	}
	return {
		/** Read-only, and also while the rules have not been read yet: the preview's writes wait
		 * for the lookup rather than race it. */
		get current() {
			const ws = workspace()
			return !!ws && (verdictFor(ws) ?? true)
		},
		/** Read-only as far as is known, so false while the rules are read. For what only shows the
		 * verdict, the picker's lock or a label, which would otherwise flash a reason that is not
		 * true. */
		get known() {
			const ws = workspace()
			return !!ws && (verdictFor(ws) ?? false)
		},
		/** Protected by a rule that was actually read: false when the read failed, where the session
		 * is still read-only. For text that names the rule as the reason. */
		get confirmed() {
			const ws = workspace()
			if (!ws) return false
			const landed = verdict.current
			return landed?.workspace === ws && landed.direct === direct()
				? landed.readOnly && landed.rulesRead
				: !direct() && (settledReadOnlyVerdict(ws) ?? false)
		},
		/** Whether the rules were read, a failed read included, which settles as read-only. */
		get settled() {
			const ws = workspace()
			return !!ws && verdictFor(ws) !== undefined
		}
	}
}

/** The session chat whose work mode the session UI follows. */
type WorkModeHost = {
	readonly workMode: AIWorkMode
	readonly directIn?: string
	readonly moving?: boolean
	readonly moveStarted?: boolean
}

/**
 * What the session UI may offer in `workspace`. Its editors save drafts, so it reads
 * `roleCanDraft` / `canDraftItem` — the rule the chat's draft tools and `open_preview` follow — so
 * a control and the tool behind it never disagree. An unknown user reads as able to draft, as the toolset does before access
 * resolves: hiding every editor from a developer while `whoami` is in flight is the worse
 * mistake, and the server refuses an operator anyway. A session held in Use opens no editor: one
 * in a protected workspace, one whose role builds nothing there, or one whose chat is in Use
 * mode — the chat `chat` returns, else the `aiChatManager` context's. One moving into a fork opens
 * no draft editor until the move ends, so nothing saves a draft in the workspace being left.
 * Reads context and registers an effect: call during component initialisation.
 */
export function useSessionPermissions(
	workspace: () => string | undefined,
	chat?: () => WorkModeHost | undefined
) {
	const contextChat = getContext<WorkModeHost | undefined>('aiChatManager')
	const rights = useEditRights(workspace)
	const user = useActingUser(workspace)
	const host = $derived(chat?.() ?? contextChat)
	const protectedWorkspace = useSessionReadOnly(
		workspace,
		() => !!host?.directIn && host.directIn === workspace()
	)
	const readOnly = $derived(
		protectedWorkspace.current ||
			!buildsAnything(rights.capabilities) ||
			host?.workMode === AIWorkMode.USE
	)
	const moving = $derived(!!host?.moving)
	const moveStarted = $derived(!!host?.moveStarted)
	const workspaces = fromStore(userWorkspaces)
	const entry = $derived(workspaces.current.find((w) => w.id === workspace()))

	return {
		/** The session is read-only in this workspace. */
		get readOnly() {
			return readOnly
		},
		/** The session is moving into a fork: its draft editors stay closed until it ends. */
		get moving() {
			return moving
		},
		/** A move has started: the preview takes no more input, so what its editors hold can be
		 * drained before `moving` closes them. A keystroke after the drain would be lost with the
		 * editor. */
		get moveStarted() {
			return moveStarted
		},
		/** Whether `canEditItem` answers for real rather than refusing while it waits for this
		 *  user's rights or the protection rules. */
		get settled() {
			return user.resolved(workspace()) && protectedWorkspace.settled
		},
		/** Whether this user can save the drafts the `kind` editor writes. */
		canOpenEditor: (kind: string) => !readOnly && !moving && rights.roleCanDraft(kind),
		/** The same, and this item's own permissions let them write it. */
		canEditItem: (item: Pick<WorkspaceItem, 'kind' | 'raw_app' | 'path' | 'extraPerms'>) =>
			!readOnly &&
			!moving &&
			rights.canDraftItem(item.raw_app ? 'raw_app' : item.kind, item.path, item.extraPerms),
		/** Whether the page loads for this user rather than refusing them. */
		pageAllowed: (page: PreviewPage) =>
			// A read-only session reaches what an operator reaches: no admin page.
			!(readOnly && page.adminOnly) && previewPageAllowed(page, user.current, entry)
	}
}
