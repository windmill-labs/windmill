import { z } from 'zod'
import type { ApplyWorkspaceSettingsDocumentResponse } from '$lib/gen'
import { createToolDef, type ToolCodeDiff } from '../shared'
import type { SessionTool, SessionToolPolicy } from '../sessionCapabilities'
import { toStableYaml } from './draftDiff'
import { runOnServer, runServerTool, type SessionToolEffect } from './backendSessionTools'

// Both endpoints call `require_admin`.
const ADMIN: SessionToolPolicy = ['admin']


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
			const { text } = await runOnServer('get_workspace_settings', workspace, args)
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
		confirmAction: 'apply',
		confirmationMessage: 'Change workspace settings',
		cancellationMessage: 'The user declined: no setting was changed.',
		showDetails: true,
		validateBeforeConfirmation: async ({ args, workspace }) => {
			setWorkspaceSettingsSchema.parse(args)
			const { text, effects } = await runOnServer('set_workspace_settings', workspace, args, {
				check_only: true
			})
			const facts = settingsFacts(effects)
			if (facts.refused) {
				return { label: facts.refused, result: text }
			}
			dryRuns.set(args, facts.changes ?? [])
			return undefined
		},
		onConfirmationRequested: ({ args, toolCallbacks, toolId }) => {
			const changes = dryRuns.get(args)
			if (changes) {
				toolCallbacks.setToolStatus(toolId, { codeDiff: changesDiff(changes) })
			}
		},
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			setWorkspaceSettingsSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: 'Changing workspace settings...' })
			const { text, effects } = await runServerTool('set_workspace_settings', {
				args,
				workspace,
				toolId,
				toolCallbacks
			})
			const { changes } = settingsFacts(effects)
			if (changes) {
				toolCallbacks.setToolStatus(toolId, {
					content: `Changed workspace settings: ${changes.map((c) => c.key).join(', ')}`,
					codeDiff: changesDiff(changes)
				})
			}
			return text
		}
	}
]

function settingsFacts(effects: SessionToolEffect[]): {
	refused?: string
	changes?: SettingsChange[]
} {
	const facts = effects.find((e) => e.type === 'facts') as Record<string, unknown> | undefined
	return {
		refused: facts?.refused as string | undefined,
		changes: facts?.changes as SettingsChange[] | undefined
	}
}
