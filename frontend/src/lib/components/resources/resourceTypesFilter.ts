import { Boxes, FileText } from 'lucide-svelte'
import type { FilterSchemaRec } from '../FilterSearchbar.svelte'

export function buildResourceTypesFilterSchema() {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		name: {
			type: 'string' as const,
			label: 'Name',
			icon: Boxes,
			description: 'Search in resource type name'
		},
		description: {
			type: 'string' as const,
			label: 'Description',
			icon: FileText,
			description: 'Search in resource type description'
		}
	} satisfies FilterSchemaRec
}
