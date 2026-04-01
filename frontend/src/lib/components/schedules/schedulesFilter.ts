import { FileCode, FileText, Clock, Braces, Tag, Users } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildSchedulesFilterSchema({
	paths,
	scriptPaths,
	labels,
	showUserFoldersFilter,
	userFoldersLabel
}: {
	paths: string[]
	scriptPaths: string[]
	labels?: string[]
	showUserFoldersFilter?: boolean
	userFoldersLabel?: string
}) {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		schedule_path: {
			type: 'oneof' as const,
			options: paths.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: false,
			allowMultiple: false,
			label: 'Schedule path',
			icon: Clock,
			description: 'Filter by exact schedule path'
		},
		path_start: {
			type: 'string' as const,
			label: 'Path prefix',
			icon: FileCode,
			description: 'Filter by schedule path prefix'
		},
		path: {
			type: 'oneof' as const,
			options: scriptPaths.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: false,
			allowMultiple: false,
			label: 'Script/Flow path',
			icon: FileCode,
			description: 'Filter by the script or flow path that the schedule runs'
		},
		description: {
			type: 'string' as const,
			label: 'Description',
			icon: FileText,
			description: 'Search in schedule description'
		},
		summary: {
			type: 'string' as const,
			label: 'Summary',
			icon: FileText,
			description: 'Search in schedule summary'
		},
		args: {
			type: 'string' as const,
			format: 'json' as const,
			label: 'Args subset',
			icon: Braces,
			description: 'Filter by JSON args subset match (e.g., {"param": "value"})'
		},
		label: {
			type: 'oneof' as const,
			options: (labels ?? []).map((s) => ({ label: s, value: s })),
			allowNegative: false,
			allowMultiple: true,
			label: 'Label',
			icon: Tag,
			description: 'Filter by label (comma-separated for multiple)'
		},
		...(showUserFoldersFilter
			? {
					user_folders_only: {
						type: 'boolean' as const,
						label: userFoldersLabel || 'User folders only',
						icon: Users,
						description: 'Show only schedules in user folders'
					}
				}
			: {})
	} satisfies FilterSchemaRec
}
