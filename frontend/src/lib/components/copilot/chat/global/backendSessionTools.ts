/**
 * The session tools that run on the server (`backend/windmill-api/src/ai_session_tools`),
 * the same code the Windmill MCP server serves. The chat sends what only the browser
 * holds (its open editors and unsaved draft cells) and applies what the call changed
 * through its own draft store, so an open editor, its auto-save toggle and its conflict
 * dialog behave as when the browser wrote the draft itself.
 */
import { AiService } from '$lib/gen'
import type { ToolCallbacks, ToolDisplayMessage } from '../shared'
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'
import {
	deleteDraftAt,
	GLOBAL_DRAFT_KINDS,
	persistDraftAt,
	type DraftPersistResult
} from './userDraftAdapter'

export type SessionToolEffect =
	| {
			type: 'draft_write'
			kind: UserDraftItemKind
			storage_path: string
			value: unknown
			last_sync?: string | null
			force: boolean
	  }
	| {
			type: 'draft_delete'
			kind: UserDraftItemKind
			storage_path: string
			last_sync?: string | null
			keep_live_editor: boolean
	  }
	| { type: 'inspected_run'; job_id: string }
	| ({ type: 'facts' } & Record<string, unknown>)
	| ({ type: 'status'; after_save?: boolean } & Partial<ToolDisplayMessage>)
	| {
			type: 'code_diff'
			before: string
			after: string
			language?: string | null
			file_path?: string | null
			before_file_path?: string | null
	  }
	| { type: 'needs_ai_provider_catalog'; resource_refs: string[] }

export type SessionToolResponse = {
	text: string
	is_error: boolean
	effects: SessionToolEffect[]
}

/** What the browser holds that a tool reads: the editor open on each draft kind, and
 * every draft cell, which is newer than the server's row. */
export function sessionToolContext(workspace: string) {
	const live_editors = GLOBAL_DRAFT_KINDS.flatMap((kind) => {
		const editor = UserDraft.getLiveEditorDraft(kind, { workspace })
		return editor
			? [{ kind, storage_path: editor.storagePath, effective_path: editor.effectivePath }]
			: []
	})
	const cells = UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] }).map((entry) => ({
		kind: entry.itemKind,
		storage_path: entry.path,
		value: entry.value
	}))
	return { live_editors, cells }
}

export async function callSessionTool(
	workspace: string,
	name: string,
	args: unknown,
	context: Record<string, unknown> = {}
): Promise<SessionToolResponse> {
	const response = await AiService.callAiSessionTool({
		workspace,
		name,
		requestBody: { args: args ?? {}, context: { ...sessionToolContext(workspace), ...context } }
	})
	return response as SessionToolResponse
}

/** Run a session tool on the server. A failed call throws its message, as the browser's own
 * request did. */
export async function runOnServer(
	name: string,
	workspace: string,
	args: unknown,
	context?: Record<string, unknown>
): Promise<SessionToolResponse> {
	const response = await callSessionTool(workspace, name, args, context)
	if (response.is_error) throw new Error(response.text)
	return response
}

/** `runOnServer` for a tool whose card the server narrates: its status patches are applied
 * in the order the call reached them. */
export async function runServerTool(
	name: string,
	ctx: {
		args: unknown
		workspace: string
		toolId: string
		toolCallbacks: Pick<ToolCallbacks, 'setToolStatus'>
	},
	context?: Record<string, unknown>
): Promise<SessionToolResponse> {
	const response = await callSessionTool(ctx.workspace, name, ctx.args, context)
	applyStatusEffects(response.effects, ctx)
	if (response.is_error) throw new Error(response.text)
	return response
}

export function applyStatusEffects(
	effects: SessionToolEffect[],
	ctx: { toolId: string; toolCallbacks: Pick<ToolCallbacks, 'setToolStatus'> }
): void {
	for (const effect of effects) {
		if (effect.type === 'status') {
			const { type: _, after_save: __, ...patch } = effect
			ctx.toolCallbacks.setToolStatus(ctx.toolId, patch)
		}
	}
}

/** A failed delete, as the browser's own delete would have thrown it. */
export type DraftDeleteFailure = { storagePath: string; message: string }

export type AppliedEffects = {
	/** One per `draft_write`, in order. */
	writes: DraftPersistResult[]
	deleteFailure?: DraftDeleteFailure
}

/** Apply a call's effects in order. A failed delete stops there, as a throw would have. */
export async function applySessionToolEffects(
	workspace: string,
	effects: SessionToolEffect[],
	deletePath: (storagePath: string) => string = (storagePath) => storagePath
): Promise<AppliedEffects> {
	const applied: AppliedEffects = { writes: [] }
	for (const effect of effects) {
		if (effect.type === 'draft_write') {
			applied.writes.push(
				await persistDraftAt(workspace, effect.kind, effect.storage_path, effect.value, {
					force: effect.force,
					lastSync: effect.last_sync
				})
			)
		} else if (effect.type === 'draft_delete') {
			try {
				await deleteDraftAt(
					workspace,
					effect.kind,
					effect.storage_path,
					deletePath(effect.storage_path),
					{ preserveLiveDraft: effect.keep_live_editor, lastSync: effect.last_sync }
				)
			} catch (e) {
				applied.deleteFailure = {
					storagePath: effect.storage_path,
					message: e instanceof Error ? e.message : String(e)
				}
				return applied
			}
		}
	}
	return applied
}
