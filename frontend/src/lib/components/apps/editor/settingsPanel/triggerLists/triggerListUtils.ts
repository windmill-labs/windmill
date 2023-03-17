import type {
	StaticAppInput,
	ConnectedAppInput,
	RowAppInput,
	UserAppInput
} from '$lib/components/apps/inputType'

export function getDependencies(
	fields:
		| Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
		| undefined
): string[] {
	let dependencies: string[] = []

	if (!fields) return dependencies

	Object.values(fields).forEach((field) => {
		if (field.type === 'connected' && dependencies && field.connection) {
			dependencies.push(`${field.connection.componentId} - ${field.connection.path}`)
		}
	})
	return dependencies
}
