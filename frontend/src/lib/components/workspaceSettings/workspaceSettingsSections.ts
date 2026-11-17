import { isCloudHosted } from '$lib/cloud'
import { WORKSPACE_SHOW_SLACK_CMD, WORKSPACE_SHOW_WEBHOOK_CLI_SYNC } from '$lib/consts'
import type { PageHeaderSubpage } from '$lib/components/pageHeaderRegistry.svelte'
import { WORKSPACE_SETTINGS_PATH } from '$lib/components/sessions/previewPaths'

export type WorkspaceSettingsSection = {
	id: string
	label: string
	aiId: string
	aiDescription: string
	showIf?: boolean
	isEE?: boolean
}

/** The sections an admin sees in workspace settings, in sidebar order. Read outside the page as
 * well, by the session preview's picker, which lists them before the page has loaded. */
export const WORKSPACE_SETTINGS_SECTION_GROUPS: {
	title?: string
	items: WorkspaceSettingsSection[]
}[] = [
	{
		items: [
			{
				id: 'general',
				label: 'General',
				aiId: 'workspace-settings-general',
				aiDescription: 'General workspace settings'
			},
			{
				id: 'users',
				label: 'Users',
				aiId: 'workspace-settings-users',
				aiDescription: 'Users workspace settings'
			},
			{
				id: 'ai',
				label: 'Windmill AI',
				aiId: 'workspace-settings-ai',
				aiDescription: 'Windmill AI workspace settings'
			},
			{
				id: 'premium',
				label: 'Premium plans',
				aiId: 'workspace-settings-premium',
				aiDescription: 'Premium plans workspace settings',
				showIf: isCloudHosted()
			}
		]
	},
	{
		title: 'Git & deployment',
		items: [
			{
				id: 'git_sync',
				label: 'Git sync',
				aiId: 'workspace-settings-git-sync',
				aiDescription: 'Git sync workspace settings',
				isEE: true
			},
			{
				id: 'dev_workspace',
				label: 'Dev workspace',
				aiId: 'workspace-settings-dev-workspace',
				aiDescription:
					'Pair this workspace with a dev workspace (same code, different environment), and choose which items its deploy UI may promote'
			},
			{
				id: 'rulesets',
				label: 'Rulesets',
				aiId: 'workspace-settings-rulesets',
				aiDescription: 'Protection Rulesets workspace settings',
				isEE: true
			}
		]
	},
	{
		title: 'Integrations',
		items: [
			{
				id: 'slack',
				label: 'Slack / Teams',
				aiId: 'workspace-settings-slack',
				aiDescription: 'Slack / Teams workspace settings',
				showIf: WORKSPACE_SHOW_SLACK_CMD
			},
			{
				id: 'webhook',
				label: 'Webhook',
				aiId: 'workspace-settings-webhook',
				aiDescription: 'Webhook workspace settings',
				showIf: WORKSPACE_SHOW_WEBHOOK_CLI_SYNC && !isCloudHosted()
			},
			{
				id: 'native_triggers',
				label: 'Integration triggers',
				aiId: 'workspace-settings-integrations',
				aiDescription: 'Workspace integrations for integration triggers'
			}
		]
	},
	{
		title: 'Hooks',
		items: [
			{
				id: 'error_handler',
				label: 'Error / success handler',
				aiId: 'workspace-settings-error-handler',
				aiDescription: 'Error and success handler workspace settings',
				isEE: true
			},
			{
				id: 'variable_expiration',
				label: 'Variable expiration',
				aiId: 'workspace-settings-variable-expiration',
				aiDescription:
					'Runnable executed when a variable value is about to expire, so the value can be rotated'
			},
			{
				id: 'critical_alerts',
				label: 'Critical alerts',
				aiId: 'workspace-settings-critical-alerts',
				aiDescription: 'Critical alerts workspace settings',
				isEE: true
			}
		]
	},
	{
		title: 'Data & storage',
		items: [
			{
				id: 'windmill_data_tables',
				label: 'Data tables',
				aiId: 'workspace-settings-windmill-data-tables',
				aiDescription: 'Data tables workspace settings'
			},
			{
				id: 'windmill_lfs',
				label: 'Object storage (S3)',
				aiId: 'workspace-settings-windmill-lfs',
				aiDescription: 'Object Storage (S3) workspace settings'
			},
			{
				id: 'volume_storage',
				label: 'Volumes',
				aiId: 'workspace-settings-volume-storage',
				aiDescription: 'Volume storage workspace settings'
			},
			{
				id: 'ducklake',
				label: 'Ducklake',
				aiId: 'workspace-settings-ducklake',
				aiDescription: 'Ducklake workspace settings'
			},
			{
				id: 'dbt',
				label: 'dbt',
				aiId: 'workspace-settings-dbt',
				aiDescription: 'dbt warehouses workspace settings'
			}
		]
	},
	{
		title: 'Advanced',
		items: [
			{
				id: 'default_app',
				label: 'Apps',
				aiId: 'workspace-settings-apps',
				aiDescription: 'Apps workspace settings',
				isEE: true
			},
			{
				id: 'shared_ui',
				label: 'Shared UI folder',
				aiId: 'workspace-settings-shared-ui',
				aiDescription: 'Shared frontend folder usable by raw apps'
			},
			{
				id: 'dependencies',
				label: 'Dependencies',
				aiId: 'workspace-settings-dependencies',
				aiDescription: 'Workspace dependencies settings'
			},
			{
				id: 'encryption',
				label: 'Encryption',
				aiId: 'workspace-settings-encryption',
				aiDescription: 'Encryption workspace settings'
			},
			{
				id: 'trashbin',
				label: 'Trashbin',
				aiId: 'workspace-settings-trashbin',
				aiDescription: 'Trashbin for recently deleted items'
			}
		]
	}
]

/** Older `tab` values that open a section under another id. */
const SECTION_ALIASES: Record<string, string> = {
	teams: 'slack',
	success_handler: 'error_handler',
	// The Deployment UI tab was folded into Dev workspace; its links keep working.
	deploy_to: 'dev_workspace'
}

/** The section a `tab` query value opens. */
export function workspaceSettingsSectionId(tab: string): string {
	return SECTION_ALIASES[tab] ?? tab
}

/** The sections as picker subpages, each a link to `/workspace_settings?tab=<id>`. */
export function workspaceSettingsSubpages(currentTab?: string): PageHeaderSubpage[] {
	return WORKSPACE_SETTINGS_SECTION_GROUPS.flatMap((group) =>
		group.items
			.filter((i) => i.showIf !== false)
			.map((i) => ({
				label: i.label,
				href: `${WORKSPACE_SETTINGS_PATH}?tab=${i.id}`,
				group: group.title,
				current: i.id === currentTab
			}))
	)
}
