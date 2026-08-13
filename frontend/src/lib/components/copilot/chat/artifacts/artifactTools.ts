import { z } from 'zod'
import { createToolDef, type Tool } from '../shared'
import { currentVersion, type ArtifactVersion } from './artifactsDB'
import type { SessionArtifactsStore } from './artifactsState.svelte'

// The subset of GlobalToolHelpers these tools read. Kept local (not imported from
// global/core) so the tools don't pull the whole global tool module — which would be a
// circular import, since global/core registers these tools.
type ArtifactToolHelpers = {
	artifacts?: SessionArtifactsStore
	sessionId?: string
	getChatId?: () => string | undefined
	openArtifact?: (artifactId: string, name: string) => void
}

const MAX_ARTIFACT_BYTES = 256 * 1024

// Bounds what a snapshot stores and replays to the model. Enforced by truncation rather
// than by the schema: rejecting the call would throw away a real content update over a
// cosmetic label, and the model would have to resend the whole document to recover.
const MAX_NOTE_CHARS = 120

const createArtifactSchema = z.object({
	name: z.string().describe('Short display title for the artifact.'),
	content: z.string().describe('Full markdown content of the artifact.')
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
	const bytes = new TextEncoder().encode(content).length
	if (bytes <= MAX_ARTIFACT_BYTES) return undefined
	return `Content is too large (${bytes} bytes, limit ${MAX_ARTIFACT_BYTES}). Shorten or split it.`
}

const UNAVAILABLE = 'Artifacts are only available inside an AI session.'

export const artifactTools: Tool<{}>[] = [
	{
		def: createToolDef(
			createArtifactSchema,
			'create_artifact',
			'Create a markdown artifact in the current session.'
		),
		showDetails: true,
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
			const artifact = await h.artifacts.create(sessionId, {
				name: parsed.name,
				content: parsed.content,
				kind: 'md',
				chatId: h.getChatId?.()
			})
			h.openArtifact?.(artifact.id, artifact.name)
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
			const updated = await h.artifacts.update(
				parsed.id,
				{
					content: parsed.content,
					name: parsed.name,
					// Blank collapses to undefined, not "": an empty string is not nullish, so it
					// would slip past the picker's fallback and render a row with no label.
					note: parsed.change_note.trim().slice(0, MAX_NOTE_CHARS) || undefined
				},
				{ sessionId }
			)
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
			"List the current session's artifacts (id, name, kind)."
		),
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
					.map((a) => ({ id: a.id, name: a.name, kind: a.kind }))
			)
		}
	},
	{
		def: createToolDef(
			readArtifactSchema,
			'read_artifact',
			"Read an artifact's full markdown content by id, at its current or an earlier version."
		),
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
