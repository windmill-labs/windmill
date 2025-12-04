import type { ComponentType } from 'svelte'
import { BookOpen, Users, Workflow, GraduationCap, Wrench } from 'lucide-svelte'
import { base } from '$lib/base'

export interface TutorialConfig {
	id: string
	icon: ComponentType
	title: string
	description: string
	onClick: () => void
	index?: number // Bitmask index in the database (for progress tracking)
	disabled?: boolean
	comingSoon?: boolean
	requiredRole?: 'admin' | 'developer' | 'operator'
	order?: number
}

export interface TabConfig {
	label: string
	icon: ComponentType
	tutorials: TutorialConfig[]
}

export type TabId = 'quickstart' | 'team'

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

export const TUTORIALS_CONFIG: Record<TabId, TabConfig> = {
	quickstart: {
		label: 'Quickstart',
		icon: BookOpen,
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
				disabled: false,
				comingSoon: false,
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
				disabled: false,
				comingSoon: false,
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
				disabled: false,
				comingSoon: false,
				order: 3
			}
		]
	},
	team: {
		label: 'Team Collaboration',
		icon: Users,
		tutorials: []
	}
} as const

