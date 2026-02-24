import { FileCode, FileText, Clock, Braces, Users } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildSchedulesFilterSchema({
	paths,
	scriptPaths,
	showUserFoldersFilter,
	userFoldersLabel
}: {
	paths: string[]
	scriptPaths: string[]
	showUserFoldersFilter?: boolean
	userFoldersLabel?: string
}) {
	return {
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
