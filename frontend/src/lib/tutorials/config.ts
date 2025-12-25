import type { ComponentType } from 'svelte'
import { Workflow, GraduationCap, Wrench, PlayCircle, Link2, History } from 'lucide-svelte'
import { base } from '$lib/base'
import type { Role } from './roleUtils'

export interface TutorialConfig {
	id: string
	icon: ComponentType
	title: string
	description: string
	onClick: () => void
	index?: number // Bitmask index in the database (for progress tracking)
	active?: boolean // Whether this tutorial is active and should be displayed (default: true)
	comingSoon?: boolean
	roles?: Role[] // Roles that can access this tutorial (if not specified, available to everyone)
	order?: number
}

export interface TabConfig {
	label: string
	tutorials: TutorialConfig[]
	roles?: Role[] // Roles that can access this tab category (if not specified, available to everyone)
	progressBar?: boolean // Whether to display the progress bar for this tab (default: true)
	active?: boolean // Whether this tab category is active and should be displayed (default: true)
}

export type TabId = 'quickstart' | 'app_editor'

/**
 * Get tutorial index from config by tutorial ID.
 * Throws an error if the tutorial or its index is not found.
 */
export function getTutorialIndex(id: string): number {
	for (const tab of Object.values(TUTORIALS_CONFIG)) {
		const tutorial = tab.tutorials.find((t) => t.id === id)
		if (tutorial?.index !== undefined) return tutorial.index
	}
	throw new Error(`Tutorial index not found for id: ${id}. Make sure the tutorial has an index defined in config.`)
}

// Available roles : developer, admin, operator

export const TUTORIALS_CONFIG: Record<TabId, TabConfig> = {
	quickstart: {
		label: 'Quickstart',
		roles: ['admin', 'developer', 'operator'],
		progressBar: true,
		active: true,
		tutorials: [
			{
				id: 'workspace-onboarding',
				icon: GraduationCap,
				title: 'Workspace onboarding',
				description: 'Discover the basics of Windmill with a quick tour of the workspace.',
				onClick: () => {
					window.location.href = `${base}/?tutorial=workspace-onboarding`
				},
				index: 1,
				active: true,
				comingSoon: false,
				roles: ['developer', 'admin'],
				order: 1
			},
			{
				id: 'flow-live-tutorial',
				icon: Workflow,
				title: 'Build a flow',
				description: 'Learn how to build workflows in Windmill with our interactive tutorial.',
				onClick: () => {
					window.location.href = `${base}/flows/add?tutorial=flow-live-tutorial&nodraft=true`
				},
				index: 2,
				active: true,
				comingSoon: false,
				roles: ['developer', 'admin'],
				order: 2
			},
			{
				id: 'troubleshoot-flow',
				icon: Wrench,
				title: 'Fix a broken flow',
				description: 'Learn how to monitor and debug your script and flow executions.',
				onClick: () => {
					window.location.href = `${base}/flows/add?tutorial=troubleshoot-flow&nodraft=true`
				},
				index: 3,
				active: true,
				comingSoon: false,
				roles: ['admin','developer'],
				order: 3
			},
			{
				id: 'runs-tutorial',
				icon: History,
				title: 'Discover your monitoring dashboard',
				description: 'Learn how to monitor, filter, and manage your script and flow executions.',
				onClick: () => {
					window.location.href = `${base}/runs?tutorial=runs-tutorial`
				},
				index: 7,
				active: true,
				comingSoon: false,
				roles: ['admin', 'developer','operator'],
				order: 4
			},
			{
				id: 'workspace-onboarding-operator',
				icon: GraduationCap,
				title: 'Workspace onboarding',
				description: 'Discover the basics of Windmill with a quick tour of the workspace.',
				onClick: () => {
					window.location.href = `${base}/?tutorial=workspace-onboarding-operator`
				},
				index: 6,
				active: true,
				comingSoon: false,
				roles: ['operator'],
				order: 1
			},	
		]
	},
	app_editor: {
		label: 'App Editor',
		roles: ['developer', 'admin'],
		progressBar: false,
		active: true,
		tutorials: [
            {
				id: 'backgroundrunnables',
				icon: PlayCircle,
				title: 'Background runnables',
				description: 'Learn how to create and use background runnables in your apps.',
				onClick: () => {
					window.location.href = `${base}/apps/add?tutorial=backgroundrunnables&nodraft=true`
				},
				index: 4,
				active: true,
				comingSoon: false,
				roles: ['developer','admin'],
				order: 4
			},
			{
				id: 'connection',
				icon: Link2,
				title: 'Connection',
				description: 'Learn how to connect component inputs to outputs in your apps.',
				onClick: () => {
					window.location.href = `${base}/apps/add?tutorial=connection&nodraft=true`
				},
				index: 5,
				active: true,
				comingSoon: false,
				roles: ['developer', 'admin'],
				order: 5
			}
        ]
	}
} as const

