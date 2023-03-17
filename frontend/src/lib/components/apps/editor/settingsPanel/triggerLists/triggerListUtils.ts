import type {
	StaticAppInput,
	ConnectedAppInput,
	RowAppInput,
	UserAppInput
} from '$lib/components/apps/inputType'

type Dependencies = Array<{ componentId: string; path: string }>

export function getDependencies(
	fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
): Dependencies {
	let dependencies: Dependencies = []
	Object.values(fields).forEach((field) => {
		if (field.type === 'connected' && dependencies && field.connection) {
			dependencies.push({
				componentId: field.connection?.componentId,
				path: field.connection?.path
			})
		}
	})
	return dependencies
}

export type DependencyBadge = Array<{ label: string; color: string }>
