import type { ErrorHandler } from '$lib/gen'
import { WORKSPACE_SLACK_BOT_TOKEN_PATH } from '$lib/consts'
import hubPaths from '$lib/hubPaths.json'

export type HandlerMode = 'error' | 'recovery' | 'success' | 'variableExpiration'
type BuiltinDestination = 'slack' | 'teams' | 'email'

export type HandlerSlot = { path?: string; args: Record<string, any> }
export type HandlerDraft = {
	selected: ErrorHandler
	slack: HandlerSlot
	teams: HandlerSlot
	email: HandlerSlot
	custom: HandlerSlot & { kind: 'script' | 'flow' }
}
type StoredHandler = { path?: string; kind: 'script' | 'flow'; args: Record<string, any> }

const SLACK_BOT_ARG = '$res:' + WORKSPACE_SLACK_BOT_TOKEN_PATH

// Suffixes rather than full paths, so a handler stored under an older hub id is still recognised.
// Email uses one script for every mode.
const EMAIL_SUFFIX = '/workspace-or-error-handler-email'
const BUILTIN_SUFFIXES: Record<HandlerMode, Record<'slack' | 'teams', string>> = {
	error: {
		slack: '/workspace-or-schedule-error-handler-slack',
		teams: '/workspace-or-schedule-error-handler-teams'
	},
	recovery: {
		slack: '/schedule-recovery-handler-slack',
		teams: '/schedule-recovery-handler-teams'
	},
	success: {
		slack: '/schedule-success-handler-slack',
		teams: '/schedule-success-handler-teams'
	},
	variableExpiration: {
		slack: '/variable-expiration-handler-slack',
		teams: '/variable-expiration-handler-teams'
	}
}

// The only settings each built-in hub script takes besides the ones the backend injects.
const BUILTIN_KEYS: Record<BuiltinDestination, string[]> = {
	slack: ['channel'],
	teams: ['channel', 'channel_name'],
	email: ['email_recipients']
}

export function builtinHubPath(mode: HandlerMode, destination: 'slack' | 'teams'): string {
	const key =
		`${destination}${mode[0].toUpperCase()}${mode.slice(1)}Handler` as keyof typeof hubPaths
	return hubPaths[key]
}

export function handlerTypeForPath(mode: HandlerMode, path: string | undefined): ErrorHandler {
	if (path?.startsWith('hub/')) {
		if (path.endsWith(EMAIL_SUFFIX)) return 'email'
		for (const [destination, suffix] of Object.entries(BUILTIN_SUFFIXES[mode])) {
			if (path.endsWith(suffix)) return destination as BuiltinDestination
		}
	}
	return 'custom'
}

function emptyDraft(selected: ErrorHandler): HandlerDraft {
	return {
		selected,
		slack: { args: {} },
		teams: { args: {} },
		email: { path: hubPaths.emailErrorHandler, args: {} },
		custom: { kind: 'script', args: {} }
	}
}

function pick(args: Record<string, any>, keys: string[]): Record<string, any> {
	return Object.fromEntries(keys.filter((k) => args[k] !== undefined).map((k) => [k, args[k]]))
}

/** Puts a stored handler into the slot of its destination; every other slot starts empty. */
export function handlerDraftFromStored(
	mode: HandlerMode,
	path: string | undefined,
	kind: 'script' | 'flow' = 'script',
	args: Record<string, any> | null | undefined = {},
	opts: { asCustom?: boolean; emptySelected?: ErrorHandler } = {}
): HandlerDraft {
	if (!path) return emptyDraft(opts.emptySelected ?? 'custom')
	const destination = opts.asCustom ? 'custom' : handlerTypeForPath(mode, path)
	const draft = emptyDraft(destination)
	if (destination === 'custom') {
		draft.custom = { path, kind, args: { ...(args ?? {}) } }
	} else if (destination !== 'instance_alerts') {
		draft[destination] = { path, args: pick(args ?? {}, BUILTIN_KEYS[destination]) }
	}
	return draft
}

/** What a save stores: the selected destination only. */
export function handlerDraftToStored(draft: HandlerDraft): StoredHandler {
	const none: StoredHandler = { kind: 'script', args: {} }
	switch (draft.selected) {
		case 'custom': {
			const { path, kind, args } = draft.custom
			return path ? { path, kind, args: $state.snapshot(args) } : none
		}
		case 'slack':
		case 'teams':
		case 'email': {
			const { path, args } = draft[draft.selected]
			if (!path) return none
			const stored = { path, kind: 'script' as const, args: $state.snapshot(args) }
			if (draft.selected === 'slack') stored.args.slack = SLACK_BOT_ARG
			return stored
		}
		default:
			return none
	}
}

export function cloneHandlerDraft(draft: HandlerDraft): HandlerDraft {
	return $state.snapshot(draft) as HandlerDraft
}
