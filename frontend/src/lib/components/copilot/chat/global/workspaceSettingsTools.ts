import { z } from 'zod'
import YAML from 'yaml'
import { ApiError, WorkspaceService, type ApplyWorkspaceSettingsDocumentResponse } from '$lib/gen'
import { createToolDef, type ToolCodeDiff } from '../shared'
import type { SessionTool, SessionToolPolicy } from '../sessionCapabilities'
import { toStableYaml } from './draftDiff'

// Both endpoints call `require_admin`.
const ADMIN: SessionToolPolicy = ['admin']

const MAX_READ_CHARS = 20_000

type SettingsChange = ApplyWorkspaceSettingsDocumentResponse['changes'][number]

const getWorkspaceSettingsSchema = z.object({
	keys: z
		.array(z.string())
		.optional()
		.describe('Only these top-level keys. Omit for the whole document.')
})

const setWorkspaceSettingsSchema = z.object({
	yaml: z
		.string()
		.describe(
			'A YAML mapping of only the top-level keys to change, each given whole (a nested object replaces the stored one). A key set to null clears it.'
		)
})

function apiErrorText(e: unknown): string {
	if (e instanceof ApiError) {
		return typeof e.body === 'string' ? e.body : JSON.stringify(e.body)
	}
	return e instanceof Error ? e.message : String(e)
}

function parseSettingsYaml(text: string): Record<string, unknown> {
	const doc = YAML.parse(text)
	if (doc === null || typeof doc !== 'object' || Array.isArray(doc)) {
		throw new Error('The yaml must be a mapping of setting keys to values.')
	}
	return doc
}

// An unset value is left out rather than written as null, so a side with nothing set is empty.
function changesSide(changes: SettingsChange[], side: 'before' | 'after'): string {
	const set = changes.filter((c) => c[side] != null)
	return set.length ? toStableYaml(Object.fromEntries(set.map((c) => [c.key, c[side]]))) : ''
}

function changesDiff(changes: SettingsChange[]): ToolCodeDiff {
	return {
		before: changesSide(changes, 'before'),
		after: changesSide(changes, 'after'),
		lang: 'yaml'
	}
}

// The dry run answers before the confirmation card, and the card shows its changes. Keyed by
// the call's parsed arguments, the one object both hooks are handed.
const dryRuns = new WeakMap<object, SettingsChange[]>()

export const workspaceSettingsTools: SessionTool<{}>[] = [
	{
		requires: ADMIN,
		def: createToolDef(
			getWorkspaceSettingsSchema,
			'get_workspace_settings',
			'Read the workspace settings as YAML, in the layout of the settings.yaml file the CLI syncs. Secrets read as `__SENSITIVE_AND_UNCHANGED__`.'
		),
		planModeSafe: true,
		showDetails: true,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsed = getWorkspaceSettingsSchema.parse(args)
			const doc = (await WorkspaceService.getWorkspaceSettingsDocument({
				workspace,
				secrets: 'redacted'
			})) as Record<string, unknown>
			let picked = doc
			let note = ''
			if (parsed.keys?.length) {
				picked = Object.fromEntries(parsed.keys.filter((k) => k in doc).map((k) => [k, doc[k]]))
				const unset = parsed.keys.filter((k) => !(k in doc))
				if (unset.length) {
					note = `\n# Not in the document (unset, or not a setting): ${unset.join(', ')}`
				}
			}
			let text = YAML.stringify(picked, { aliasDuplicateObjects: false, lineWidth: 0 }) + note
			if (text.length > MAX_READ_CHARS) {
				text = `${text.slice(0, MAX_READ_CHARS)}\n… truncated at ${MAX_READ_CHARS} chars; read the keys you need with \`keys\`.`
			}
			toolCallbacks.setToolStatus(toolId, {
				content: parsed.keys?.length
					? `Read workspace settings: ${parsed.keys.join(', ')}`
					: 'Read workspace settings'
			})
			return text
		}
	},
	{
		requires: ADMIN,
		def: createToolDef(
			setWorkspaceSettingsSchema,
			'set_workspace_settings',
			'Change workspace settings. Keys the yaml omits are left as they are. The user reviews the changes before they apply. Read get_instructions subject "workspace_settings" before your first change.'
		),
		requiresConfirmation: true,
		confirmationMessage: 'Change workspace settings',
		cancellationMessage: 'The user declined: no setting was changed.',
		showDetails: true,
		validateBeforeConfirmation: async ({ args, workspace }) => {
			let changes: SettingsChange[]
			let warnings: string[] | undefined
			try {
				const document = parseSettingsYaml(setWorkspaceSettingsSchema.parse(args).yaml)
				;({ changes, warnings } = await WorkspaceService.applyWorkspaceSettingsDocument({
					workspace,
					requestBody: { document, replace: false, dry_run: true }
				}))
			} catch (e) {
				const error = apiErrorText(e)
				return { label: `Invalid settings: ${error}`, result: `Nothing was changed: ${error}` }
			}
			if (changes.length === 0) {
				return {
					label: 'No settings change',
					result: `The settings already hold these values: nothing to change.${
						warnings?.length ? ` Warnings: ${warnings.join(' ')}` : ''
					}`
				}
			}
			dryRuns.set(args, changes)
			return undefined
		},
		onConfirmationRequested: ({ args, toolCallbacks, toolId }) => {
			const changes = dryRuns.get(args)
			if (changes) {
				toolCallbacks.setToolStatus(toolId, { codeDiff: changesDiff(changes) })
			}
		},
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const document = parseSettingsYaml(setWorkspaceSettingsSchema.parse(args).yaml)
			toolCallbacks.setToolStatus(toolId, { content: 'Changing workspace settings...' })
			try {
				const { changes, warnings } = await WorkspaceService.applyWorkspaceSettingsDocument({
					workspace,
					requestBody: { document, replace: false }
				})
				const keys = changes.map((c) => c.key)
				toolCallbacks.setToolStatus(toolId, {
					content: `Changed workspace settings: ${keys.join(', ')}`,
					codeDiff: changesDiff(changes)
				})
				return JSON.stringify({
					success: true,
					message: `The user approved and these settings are now applied: ${keys.join(', ')}.`,
					...(warnings?.length ? { warnings } : {})
				})
			} catch (e) {
				const error = apiErrorText(e)
				toolCallbacks.setToolStatus(toolId, {
					content: 'Failed to change workspace settings',
					error
				})
				return JSON.stringify({ success: false, error })
			}
		}
	}
]
