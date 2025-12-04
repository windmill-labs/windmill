import type { ComponentType } from 'svelte'
import { BookOpen, Users, Workflow, GraduationCap, Wrench } from 'lucide-svelte'
import { base } from '$lib/base'

export interface TutorialConfig {
	id: string
	icon: ComponentType
	title: string
	description: string
	onClick: () => void
}

export interface TabConfig {
	label: string
	icon: ComponentType
	tutorials: TutorialConfig[]
}

export type TabId = 'quickstart' | 'team'

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
				}
			},
			{
				id: 'flow-live-tutorial',
				icon: Workflow,
				title: 'Build a flow',
				description: 'Learn how to build workflows in Windmill with our interactive tutorial.',
				onClick: () => {
					window.location.href = `${base}/flows/add?tutorial=flow-live-tutorial&nodraft=true`
				}
			},
			{
				id: 'troubleshoot-flow',
				icon: Wrench,
				title: 'Fix a broken flow',
				description: 'Learn how to monitor and debug your script and flow executions.',
				onClick: () => {
					window.location.href = `${base}/flows/add?tutorial=troubleshoot-flow&nodraft=true`
				}
			}
		]
	},
	team: {
		label: 'Team Collaboration',
		icon: Users,
		tutorials: []
	}
} as const

