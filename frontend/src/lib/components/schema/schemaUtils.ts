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

export function getFullPath(arg: { label: string; nestedParent: any | undefined }): string[] {
	const getPath = (current: { label: string; nestedParent: any | undefined }): string[] => {
		if (!current.nestedParent) {
			return [current.label]
		}
		return [...getPath(current.nestedParent), current.label]
	}

	return getPath(arg)
}

export function getNestedProperty(obj: any, path: string[], field: string = 'properties') {
	return path.reduce((curr, key) => curr?.[field]?.[key], obj)
}

export function setNestedProperty(
	obj: any,
	path: string[],
	value: any,
	field: string = 'properties'
) {
	const pathCopy = [...path]
	const lastKey = pathCopy.pop()
	const target = pathCopy.reduce((curr, key) => {
		if (!(key in curr[field])) {
			curr[field][key] = { [field]: {} }
		}
		return curr[field][key]
	}, obj)
	if (lastKey && value) {
		const newValue = structuredClone(value)
		target[field][lastKey] = newValue
		return
	}
	if (lastKey && !value) {
		delete target[field][lastKey]
	}
}

export function getNestedOrder(obj: any, path: string[]) {
	if (path.length === 0) return obj.order
	return path.reduce((curr, key) => curr?.properties?.[key], obj)?.order
}

export function setNestedOrder(obj: any, path: string[], value: string[]) {
	if (path.length === 0) {
		obj.order = value
		return
	}
	const target = path.reduce((curr, key) => curr?.properties?.[key], obj)
	if (target) {
		target.order = value
	}
}
