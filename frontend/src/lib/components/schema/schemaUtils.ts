export type SchemaDiff = {
	diff: 'same' | 'added' | 'removed' | 'modified' | Record<string, SchemaDiff>
	fullSchema: { [key: string]: any } | undefined
}

export function computeSchemaDiff(
	previewSchema: { [key: string]: any } | undefined,
	currentSchema: { [key: string]: any } | undefined
) {
	if (!previewSchema || !currentSchema) {
		return { diffSchema: {}, fullSchema: undefined }
	}
	const diffSchema: Record<string, SchemaDiff> = {}
	const fullSchema = structuredClone(currentSchema)

	if (previewSchema?.properties) {
		Object.keys(previewSchema.properties).forEach((key) => {
			if (!currentSchema?.properties?.[key]) {
				diffSchema[key] = {
					diff: 'added',
					fullSchema: undefined
				}
				fullSchema.properties[key] = structuredClone(previewSchema.properties[key])
				fullSchema.order.push(key)
				//TODO: add other properties
			} else {
				const previewProp = previewSchema.properties[key]
				const currentProp = currentSchema.properties[key]
				if (JSON.stringify(previewProp) === JSON.stringify(currentProp)) {
					diffSchema[key] = { diff: 'same', fullSchema: undefined }
				} else if (previewProp.type === 'object' && currentProp.type === 'object') {
					const { diffSchema: diffProp, fullSchema: fullProp } = computeSchemaDiff(
						previewProp,
						currentProp
					)
					diffSchema[key] = { diff: diffProp, fullSchema: fullProp }
					fullSchema.properties[key] = fullProp
				} else {
					diffSchema[key] = { diff: 'modified', fullSchema: currentProp }
					fullSchema.properties[key] = structuredClone(previewProp)
				}
			}
		})
	}

	if (currentSchema?.properties) {
		Object.keys(currentSchema.properties).forEach((key) => {
			if (!previewSchema?.properties?.[key]) {
				diffSchema[key] = { diff: 'removed', fullSchema: undefined }
			}
		})
	}

	return { diffSchema, fullSchema }
}
