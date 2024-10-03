export type Changelog = {
	label: string
	href: string
	date: string
}

const changelogs: Changelog[] = [
	{
		label: 'Custom HTTP routes',
		href: 'https://www.windmill.dev/changelog/http-routing',
		date: '2024-09-23'
	},
	{
		label: 'Set/Get progress from code',
		href: 'https://www.windmill.dev/changelog/explicit-progress',
		date: '2024-09-18'
	},
	{
		label: 'Directly edit flow YAML',
		href: 'https://www.windmill.dev/changelog/flow-yaml-editor',
		date: '2024-09-02'
	},
	{
		label: 'Critical alert channels',
		href: 'https://www.windmill.dev/changelog/critical-alert-channels',
		date: '2024-09-01'
	},
	{
		label: 'See service logs directly in Windmill',
		href: 'https://www.windmill.dev/changelog/service-logs',
		date: '2024-09-01'
	},
	{
		label: 'Vim support for Monaco/webeditor',
		href: 'https://www.windmill.dev/changelog/vim-support',
		date: '2024-08-28'
	},
	{
		label: 'Hide / Show App Editor Panels',
		href: 'https://www.windmill.dev/changelog/hide-show-app-panels',
		date: '2024-08-26'
	},
	{
		label: 'Email triggers',
		href: 'https://www.windmill.dev/changelog/email-triggers',
		date: '2024-08-06'
	},
	{
		label: 'Continue on disapproval/timeout',
		href: 'https://www.windmill.dev/changelog/continue-on-disapproval',
		date: '2024-08-14'
	},
	{
		label: 'Nativets runtime supports npm packages and relative imports',
		href: 'https://www.windmill.dev/changelog/native-runtime-imports',
		date: '2024-07-29'
	},
	{
		label: 'App bar as components',
		href: 'https://www.windmill.dev/changelog/app-bar-components',
		date: '2024-07-29'
	},
	{
		label: 'TypeScript Bun scripts are automatically pre-bundled',
		href: 'https://www.windmill.dev/changelog/pre-bundle-bun-scripts',
		date: '2024-07-26'
	},
	{
		label: 'Dynamic Select',
		href: 'https://www.windmill.dev/changelog/dynamic-select',
		date: '2024-07-22'
	},
	{
		label: 'Flow Status Viewer improvements',
		href: 'https://www.windmill.dev/changelog/flow-status-viewer',
		date: '2024-07-14'
	},
	{
		label: 'Navbar Component',
		href: 'https://www.windmill.dev/changelog/navbar',
		date: '2024-07-05'
	},
	{
		label: 'Flow Versioning',
		href: 'https://www.windmill.dev/changelog/flow-versioning',
		date: '2024-07-04'
	},
	{
		label: 'OneOf Inputs',
		href: 'https://www.windmill.dev/changelog/oneof-inputs',
		date: '2024-06-17'
	},
	{
		label: 'Tracking relative imports to avoid dependency hell',
		href: 'https://www.windmill.dev/changelog/track-relative-imports',
		date: '2024-06-10'
	},
	{
		label: 'Windmill Customer Portal',
		href: 'https://www.windmill.dev/changelog/customer-portal',
		date: '2024-06-04'
	},
	{
		label: 'Secondary Storage',
		href: 'https://www.windmill.dev/changelog/secondary-storage',
		date: '2024-05-31'
	},
	{
		label: 'Allow User Resources in Apps with a toggle',
		href: 'https://www.windmill.dev/changelog/user-resources-in-apps',
		date: '2024-05-27'
	},
	{
		label: 'Windmill AI now supports GPT-4o',
		href: 'https://www.windmill.dev/changelog/windmill-ai-gpt-4o',
		date: '2024-05-27'
	},
	{
		label: 'Concurrency Limit Observability',
		href: 'https://www.windmill.dev/changelog/concurrency-limit-observability',
		date: '2024-05-15'
	},
	{
		label: 'Full Height Components',
		href: 'https://www.windmill.dev/changelog/full-height-components',
		date: '2024-05-15'
	},
	{
		label: 'PHP Support',
		href: 'https://www.windmill.dev/changelog/php-support',
		date: '2024-05-15'
	},
	{
		label: 'nativets/REST supports the full wmill API',
		href: 'https://www.windmill.dev/changelog/nativets-wmill-library',
		date: '2024-05-13'
	},
	{
		label: 'Workers Metrics',
		href: 'https://www.windmill.dev/changelog/workers-metrics',
		date: '2024-05-10'
	},
	{
		label: 'CLI and Git Sync major improvements',
		href: 'https://www.windmill.dev/changelog/cli-lockfiles',
		date: '2024-04-28'
	},
	{
		label: 'AgGrid Infinite Table',
		href: 'https://www.windmill.dev/changelog/aggrid-infinite-table',
		date: '2024-04-24'
	},
	{
		label: 'Jobs Labels',
		href: 'https://www.windmill.dev/changelog/jobs-labels',
		date: '2024-04-24'
	},
	{
		label: 'AgGrid Actions',
		href: 'https://www.windmill.dev/changelog/aggrid-actions',
		date: '2024-04-12'
	},
	{
		label: 'Continue on error with error as step`s return',
		href: 'https://www.windmill.dev/changelog/continue-on-error',
		date: '2024-04-02'
	},
	{
		label: 'While Loops',
		href: 'https://www.windmill.dev/changelog/while-loops',
		date: '2024-04-02'
	},
	{
		label: 'Approval Steps Improvements',
		href: 'https://www.windmill.dev/changelog/approval-steps-improvements',
		date: '2024-03-27'
	},
	{
		label: 'Map Support in Result Renderer',
		href: 'https://www.windmill.dev/changelog/map-support-renderer',
		date: '2024-03-27'
	},
	{
		label: 'Markdown support in descriptions',
		href: 'https://www.windmill.dev/changelog/markdown-support-descriptions',
		date: '2024-03-27'
	},
	{
		label: 'Custom Flow States',
		href: 'https://www.windmill.dev/changelog/custom-flow-states',
		date: '2024-03-26'
	},
	{
		label: 'Custom Contextual Variables',
		href: 'https://www.windmill.dev/changelog/custom-contextual-variables',
		date: '2024-03-23'
	},
	{
		label: 'Large log disk and Distributed storage compaction',
		href: 'https://www.windmill.dev/changelog/log-disk-distributed-storage-compaction',
		date: '2024-03-23'
	},
	{
		label: 'Rename Workspace (Self-Host only)',
		href: 'https://www.windmill.dev/changelog/rename-workspace',
		date: '2024-03-22'
	},
	{
		label: 'Configurable Available Languages',
		href: 'https://www.windmill.dev/changelog/configurable-languages',
		date: '2024-03-13'
	},
	{
		label: 'Workflow as Code',
		href: 'https://www.windmill.dev/changelog/workflows-as-code',
		date: '2024-03-04'
	},
	{
		label: 'Pin Database in SQL Scripts',
		href: 'https://www.windmill.dev/changelog/pin-database',
		date: '2024-02-27'
	},
	{
		label: 'Custom Workspace Secret Encryption',
		href: 'https://www.windmill.dev/changelog/workspace-encryption',
		date: '2024-02-15'
	},
	{
		label: 'Flow & Metadata Copilot',
		href: 'https://www.windmill.dev/changelog/ai-copilot',
		date: '2024-02-15'
	},
	{
		label: 'Ag Charts',
		href: 'https://www.windmill.dev/changelog/ag-charts',
		date: '2024-01-24'
	},
	{
		label: 'Database Studio',
		href: 'https://www.windmill.dev/changelog/database-studio',
		date: '2024-01-24'
	},
	{
		label: 'Rich results render',
		href: 'https://www.windmill.dev/changelog/rich-render',
		date: '2024-01-23'
	}
]

export { changelogs }