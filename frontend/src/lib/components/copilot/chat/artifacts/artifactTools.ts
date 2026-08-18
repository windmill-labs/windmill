import { z } from 'zod'
import { createToolDef, type Tool, type ToolCallbacks } from '../shared'
import { artifactOverflowBytes, MAX_ARTIFACT_BYTES, normalizeChangeNote } from './artifactLimits'
import {
	currentVersion,
	isPlanArtifact,
	type ArtifactVersion,
	type PersistedArtifact
} from './artifactsDB'
import type { ArtifactVersionTarget } from '$lib/components/sessions/previewRouter'
import {
	PlanSlotTakenError,
	PlanWriteRefusedError,
	type SessionArtifactsStore
} from './artifactsState.svelte'
import { PLAN_MODE_MESSAGES } from '../planModeMessages'

// The subset of GlobalToolHelpers these tools read. Kept local (not imported from
// global/core) so the tools don't pull the whole global tool module — which would be a
// circular import, since global/core registers these tools.
type ArtifactToolHelpers = {
	artifacts?: SessionArtifactsStore
	sessionId?: string
	getChatId?: () => string | undefined
	openArtifact?: (artifactId: string, name: string, version?: ArtifactVersionTarget) => void
}

const createArtifactSchema = z.object({
	name: z.string().describe('Short display title for the artifact.'),
	content: z.string().describe('Full markdown content of the artifact.'),
	role: z
		.enum(['plan'])
		.optional()
		.describe(
			"Set to `plan` to make this the session's plan document — what the user reads as the agreed plan, and what a later planning round revises. One per session: if list_artifacts already shows an entry whose `role` is `plan`, rewrite that one with update_artifact instead. Omit for an ordinary document."
		)
})

const updateArtifactSchema = z.object({
	id: z.string().describe('Id of the artifact to update, from create_artifact or list_artifacts.'),
	content: z.string().describe('New full markdown content, replacing the previous content.'),
	name: z.string().optional().describe('New display title. Omit to keep the current one.'),
	change_note: z
		.string()
		.describe(
			'What this edit changes, as a short label the user will read in the version picker: under 60 characters, no trailing period, starting with a verb — "Added rollback section", "Tightened the phase 2 wording".'
		)
})

const listArtifactsSchema = z.object({})

const readArtifactSchema = z.object({
	id: z.string().describe('Id of the artifact to read.'),
	version: z
		.number()
		.optional()
		.describe('Version to read, from list_artifact_versions. Omit for the current content.')
})

const listArtifactVersionsSchema = z.object({
	id: z.string().describe('Id of the artifact whose history to list.')
})

function tooLarge(content: string): string | undefined {
	const bytes = artifactOverflowBytes(content)
	if (bytes === undefined) return undefined
	return `Content is too large (${bytes} bytes, limit ${MAX_ARTIFACT_BYTES}). Shorten or split it.`
}

const UNAVAILABLE = 'Artifacts are only available inside an AI session.'

const PLAN_WRITE_REFUSED = {
	label: PLAN_MODE_MESSAGES.planWriteRefusedLabel,
	result: PLAN_MODE_MESSAGES.planWriteRefused
} as const

/** Report a plan write the store refused after the gate had already admitted the call. Says
 * what the gate says, so the same refusal cannot read as two different events: a lock row
 * rather than an error card, and one more block against the retry escalation. */
function reportPlanWriteRefused(toolCallbacks: ToolCallbacks, toolId: string): string {
	toolCallbacks.onToolBlockedByPlanMode?.()
	toolCallbacks.setToolStatus(toolId, {
		content: PLAN_WRITE_REFUSED.label,
		error: PLAN_WRITE_REFUSED.result,
		blockedByPlanMode: true
	})
	return JSON.stringify({ success: false, error: PLAN_WRITE_REFUSED.result })
}

/** Whether this call would write the session's plan. Synchronous because the gate is: it answers
 * from the arguments and the store's already-loaded list, never an awaited read of the database. */
function targetsPlan(args: unknown, h: ArtifactToolHelpers): boolean {
	const id = (args as { id?: unknown } | null | undefined)?.id
	if (!h.sessionId || typeof id !== 'string') return false
	return isPlanArtifact(
		{ id, role: h.artifacts?.artifacts?.find((a) => a.id === id)?.role },
		h.sessionId
	)
}

export const artifactTools: Tool<{}>[] = [
	{
		def: createToolDef(
			createArtifactSchema,
			'create_artifact',
			'Create a markdown artifact in the current session.'
		),
		showDetails: true,
		// Writes nothing outside the browser, so a research posture can keep notes and diagrams
		// here — except the one document that posture exists to put up for approval.
		planModeSafe: true,
		refuseInPlanMode: ({ args }) =>
			(args as { role?: unknown } | null | undefined)?.role === 'plan'
				? PLAN_WRITE_REFUSED
				: undefined,
		fn: async ({ args, toolId, toolCallbacks, helpers }) => {
			const parsed = createArtifactSchema.parse(args)
			const h = helpers as ArtifactToolHelpers
			const sessionId = h.sessionId
			if (!h.artifacts || !sessionId) {
				toolCallbacks.setToolStatus(toolId, { content: UNAVAILABLE, error: UNAVAILABLE })
				return JSON.stringify({ success: false, error: UNAVAILABLE })
			}
			const sizeError = tooLarge(parsed.content)
			if (sizeError) {
				toolCallbacks.setToolStatus(toolId, { content: sizeError, error: sizeError })
				return JSON.stringify({ success: false, error: sizeError })
			}
			let artifact: PersistedArtifact
			try {
				artifact = await h.artifacts.create(
					sessionId,
					{
						name: parsed.name,
						content: parsed.content,
						kind: 'md',
						chatId: h.getChatId?.(),
						role: parsed.role
						// No approvedVersion: this tool asks for no confirmation, so a plan written here
						// stands as a draft until a card decides it. exit_plan_mode alone confers it.
					},
					{ canWritePlan: () => toolCallbacks.isPlanModeActive?.() !== true }
				)
			} catch (e) {
				// The store checks both inside the write transaction, so neither a posture entered
				// mid-write nor another tab taking the slot can slip past in between.
				if (e instanceof PlanWriteRefusedError) return reportPlanWriteRefused(toolCallbacks, toolId)
				if (!(e instanceof PlanSlotTakenError)) throw e
				const error = `This session's plan is already "${e.plan.name}" (id ${e.plan.id}). Rewrite that document with update_artifact — a session holds one plan.`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			h.openArtifact?.(artifact.id, artifact.name, 'latest')
			toolCallbacks.setToolStatus(toolId, { content: `Created artifact "${artifact.name}"` })
			return JSON.stringify({ success: true, id: artifact.id, name: artifact.name })
		}
	},
	{
		def: createToolDef(
			updateArtifactSchema,
			'update_artifact',
			'Overwrite an existing markdown artifact by id.'
		),
		showDetails: true,
		planModeSafe: true,
		refuseInPlanMode: ({ args, helpers }) =>
			targetsPlan(args, helpers as ArtifactToolHelpers) ? PLAN_WRITE_REFUSED : undefined,
		fn: async ({ args, toolId, toolCallbacks, helpers }) => {
			const parsed = updateArtifactSchema.parse(args)
			const h = helpers as ArtifactToolHelpers
			const sessionId = h.sessionId
			if (!h.artifacts || !sessionId) {
				toolCallbacks.setToolStatus(toolId, { content: UNAVAILABLE, error: UNAVAILABLE })
				return JSON.stringify({ success: false, error: UNAVAILABLE })
			}
			const sizeError = tooLarge(parsed.content)
			if (sizeError) {
				toolCallbacks.setToolStatus(toolId, { content: sizeError, error: sizeError })
				return JSON.stringify({ success: false, error: sizeError })
			}
			let updated: PersistedArtifact | undefined
			try {
				updated = await h.artifacts.update(
					parsed.id,
					{
						content: parsed.content,
						name: parsed.name,
						note: normalizeChangeNote(parsed.change_note),
						// An agreed plan revised here is still agreed: this tool cannot reach the plan
						// document while plan mode is active, so any call that reaches it is one the
						// user's posture already trusts. A draft stays a draft.
						keepApproved: true
					},
					{ sessionId, canWritePlan: () => toolCallbacks.isPlanModeActive?.() !== true }
				)
			} catch (e) {
				if (!(e instanceof PlanWriteRefusedError)) throw e
				return reportPlanWriteRefused(toolCallbacks, toolId)
			}
			if (!updated) {
				const error = `No artifact found with id "${parsed.id}".`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			h.openArtifact?.(updated.id, updated.name)
			toolCallbacks.setToolStatus(toolId, { content: `Updated artifact "${updated.name}"` })
			return JSON.stringify({ success: true, id: updated.id, name: updated.name })
		}
	},
	{
		def: createToolDef(
			listArtifactsSchema,
			'list_artifacts',
			"List the current session's artifacts (id, name, kind, version, role, approvedVersion). `version` is the latest one saved, which is not necessarily the one the user is reading — get_preview_status reports the version they have pinned. `role` is `plan` on the session's one plan document and on nothing else. On that one, `approvedVersion` is the version the user signed off: below `version` means the current text is a proposal they have not agreed to, and absent means nothing here was ever approved."
		),
		planModeSafe: true,
		fn: async ({ toolId, toolCallbacks, helpers }) => {
			const h = helpers as ArtifactToolHelpers
			const sessionId = h.sessionId
			if (!h.artifacts || !sessionId) {
				toolCallbacks.setToolStatus(toolId, { content: UNAVAILABLE, error: UNAVAILABLE })
				return JSON.stringify({ success: false, error: UNAVAILABLE })
			}
			const items = await h.artifacts.listForSession(sessionId)
			toolCallbacks.setToolStatus(toolId, {
				content: `Listed ${items.length} artifact${items.length === 1 ? '' : 's'}`
			})
			return JSON.stringify(
				items
					.sort((a, b) => b.updatedAt - a.updatedAt)
					.map((a) => ({
						id: a.id,
						name: a.name,
						kind: a.kind,
						version: currentVersion(a),
						role: a.role,
						// A plan exists from the moment it is proposed, so without this the model reads
						// a refused proposal as the one they signed off.
						approvedVersion: a.role === 'plan' ? a.approvedVersion : undefined
					}))
			)
		}
	},
	{
		def: createToolDef(
			readArtifactSchema,
			'read_artifact',
			"Read an artifact's full markdown content by id, at its current or an earlier version."
		),
		planModeSafe: true,
		fn: async ({ args, toolId, toolCallbacks, helpers }) => {
			const parsed = readArtifactSchema.parse(args)
			const h = helpers as ArtifactToolHelpers
			const sessionId = h.sessionId
			if (!h.artifacts || !sessionId) {
				toolCallbacks.setToolStatus(toolId, { content: UNAVAILABLE, error: UNAVAILABLE })
				return JSON.stringify({ success: false, error: UNAVAILABLE })
			}
			const artifact = await h.artifacts.get(parsed.id)
			// An id from another session reads as absent — list_artifacts is session-scoped.
			if (!artifact || artifact.sessionId !== sessionId) {
				const error = `No artifact found with id "${parsed.id}".`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			if (parsed.version !== undefined && parsed.version !== currentVersion(artifact)) {
				let snapshot: ArtifactVersion | undefined
				try {
					snapshot = await h.artifacts.getVersion(parsed.id, parsed.version, { sessionId })
				} catch {
					// A read that failed says nothing about whether the version exists, so send the
					// model back to this same call rather than to list_artifact_versions.
					const error = `Could not read version ${parsed.version} of "${artifact.name}" — the artifact store is unavailable. Try again.`
					toolCallbacks.setToolStatus(toolId, { content: error, error })
					return JSON.stringify({ success: false, error })
				}
				if (!snapshot) {
					// Pruned or never existed; either way the model should re-list rather than retry.
					const error = `Artifact "${artifact.name}" has no version ${parsed.version}. Call list_artifact_versions for the versions still kept.`
					toolCallbacks.setToolStatus(toolId, { content: error, error })
					return JSON.stringify({ success: false, error })
				}
				toolCallbacks.setToolStatus(toolId, {
					content: `Read artifact "${artifact.name}" (v${snapshot.version})`
				})
				return JSON.stringify({
					id: artifact.id,
					name: snapshot.name,
					kind: artifact.kind,
					version: snapshot.version,
					savedAt: new Date(snapshot.savedAt).toISOString(),
					content: snapshot.content
				})
			}
			toolCallbacks.setToolStatus(toolId, { content: `Read artifact "${artifact.name}"` })
			return JSON.stringify({
				id: artifact.id,
				name: artifact.name,
				kind: artifact.kind,
				version: currentVersion(artifact),
				content: artifact.content
			})
		}
	},
	{
		def: createToolDef(
			listArtifactVersionsSchema,
			'list_artifact_versions',
			"List an artifact's saved versions, newest first. Read one with read_artifact's version argument."
		),
		// How the model recovers the approved plan once a refused draft stands in front of it;
		// the fail-closed gate would otherwise block that in the posture that needs it.
		planModeSafe: true,
		fn: async ({ args, toolId, toolCallbacks, helpers }) => {
			const parsed = listArtifactVersionsSchema.parse(args)
			const h = helpers as ArtifactToolHelpers
			const sessionId = h.sessionId
			if (!h.artifacts || !sessionId) {
				toolCallbacks.setToolStatus(toolId, { content: UNAVAILABLE, error: UNAVAILABLE })
				return JSON.stringify({ success: false, error: UNAVAILABLE })
			}
			const artifact = await h.artifacts.get(parsed.id)
			if (!artifact || artifact.sessionId !== sessionId) {
				const error = `No artifact found with id "${parsed.id}".`
				toolCallbacks.setToolStatus(toolId, { content: error, error })
				return JSON.stringify({ success: false, error })
			}
			const versions = await h.artifacts.listVersions(parsed.id, { sessionId })
			const current = currentVersion(artifact)
			toolCallbacks.setToolStatus(toolId, {
				content: `Listed ${versions.length} version${versions.length === 1 ? '' : 's'} of "${artifact.name}"`
			})
			return JSON.stringify(
				versions.map((v) => ({
					version: v.version,
					current: v.version === current,
					name: v.name,
					savedAt: new Date(v.savedAt).toISOString(),
					...(v.note ? { note: v.note } : {})
				}))
			)
		}
	}
]
