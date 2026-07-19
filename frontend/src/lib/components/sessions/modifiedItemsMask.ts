import type { UserDraftItemKind, WorkspaceItemDiff } from '$lib/gen'

// The "modified items mask" tracks which workspace items an AI chat touched via
// tool calls. Each key is `${UserDraftItemKind}:${storagePath}`. UserDraftItemKind
// is canonical because it is what `itemKindFor`/`persistGlobalDraft` emit at capture
// time and what `DraftItem.kind` (GET /drafts/list) already uses — so the mask joins
// directly against the draft list. The fork comparison (`compareWorkspaces`) uses a
// DIFFERENT kind taxonomy (`http_trigger` vs `trigger_http`, `schedule` vs
// `trigger_schedule`), so `forkDiffKindToUserDraftKind` bridges the two.

export function maskKey(kind: UserDraftItemKind, path: string): string {
	return `${kind}:${path}`
}

type ForkDiffKind = WorkspaceItemDiff['kind']

// Inverse of the draft→layout naming used in CompareDrafts.toLayoutKind. Only the
// non-identity cases need an entry; everything else falls through to identity.
// `resource_type`/`folder` have no UserDraftItemKind (never AI-authored) → undefined.
const FORK_DIFF_KIND_TO_USER_DRAFT_KIND: Partial<Record<ForkDiffKind, UserDraftItemKind>> = {
	schedule: 'trigger_schedule',
	http_trigger: 'trigger_http',
	websocket_trigger: 'trigger_websocket',
	kafka_trigger: 'trigger_kafka',
	nats_trigger: 'trigger_nats',
	postgres_trigger: 'trigger_postgres',
	mqtt_trigger: 'trigger_mqtt',
	sqs_trigger: 'trigger_sqs',
	gcp_trigger: 'trigger_gcp',
	azure_trigger: 'trigger_azure',
	email_trigger: 'trigger_default_email',
	app: 'raw_app'
}

const IDENTITY_FORK_DIFF_KINDS = new Set<ForkDiffKind>([
	'script',
	'flow',
	'raw_app',
	'resource',
	'variable'
])

export function forkDiffKindToUserDraftKind(kind: ForkDiffKind): UserDraftItemKind | undefined {
	if (IDENTITY_FORK_DIFF_KINDS.has(kind)) return kind as UserDraftItemKind
	return FORK_DIFF_KIND_TO_USER_DRAFT_KIND[kind]
}

// True when a fork-comparison diff names an item present in the chat-modified mask.
export function diffInMask(diff: WorkspaceItemDiff, mask: Set<string>): boolean {
	const kind = forkDiffKindToUserDraftKind(diff.kind)
	return kind !== undefined && mask.has(maskKey(kind, diff.path))
}
