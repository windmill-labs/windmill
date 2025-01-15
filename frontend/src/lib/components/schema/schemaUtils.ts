export type SchemaDiff = {
	diff: 'same' | 'added' | 'removed' | 'modified' | Record<string, SchemaDiff>
	fullSchema: { [key: string]: any } | undefined
	oldSchema?: { [key: string]: any } | undefined
}

export function computeDiff(
	previewSchema: { [key: string]: any } | undefined,
	currentSchema: { [key: string]: any } | undefined
) {
	if (!previewSchema || !currentSchema) {
		return {}
	}
	const diff: Record<string, SchemaDiff> = {}

	if (previewSchema?.properties) {
		Object.keys(previewSchema.properties).forEach((key) => {
			if (!currentSchema?.properties?.[key]) {
				diff[key] = {
					diff: 'added',
					fullSchema: previewSchema.properties[key]
				}
				//TODO: add other properties like required, order, etc.
			} else {
				const previewProp = previewSchema.properties[key]
				const currentProp = currentSchema.properties[key]
				if (JSON.stringify(previewProp) === JSON.stringify(currentProp)) {
					diff[key] = { diff: 'same', fullSchema: undefined }
				} else if (previewProp.type === 'object' && currentProp.type === 'object') {
					const diffProp = computeDiff(previewProp, currentProp)
					diff[key] = { diff: diffProp, fullSchema: previewProp, oldSchema: currentProp }
				} else {
					diff[key] = { diff: 'modified', fullSchema: previewProp, oldSchema: currentProp }
				}
			}
		})
	}

	if (currentSchema?.properties) {
		Object.keys(currentSchema.properties).forEach((key) => {
			if (!previewSchema?.properties?.[key]) {
				diff[key] = { diff: 'removed', fullSchema: undefined }
			}
		})
	}

	return diff
}

export function schemaFromDiff(
	diff: Record<string, SchemaDiff>,
	schema: { [key: string]: any } | undefined
) {
	if (!schema) {
		return undefined
	}
	console.log('dbg diff & schema', diff, schema)
	const newSchema = structuredClone(schema)
	Object.keys(diff).forEach((key) => {
		if (diff[key].diff === 'added' || diff[key].diff === 'modified') {
			newSchema.properties[key] = diff[key].fullSchema
			if (newSchema.order && !newSchema.order.includes(key)) {
				newSchema.order.push(key)
			}
		} else if (typeof diff[key].diff === 'object') {
			newSchema.properties[key] = schemaFromDiff(diff[key].diff, schema.properties[key])
		}
	})
	console.log('dbg updated newSchema', newSchema)
	return newSchema
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
	} else if (lastKey && !value) {
		delete target[field][lastKey]
	}
}

export function applyDiff(schema: Record<string, any>, diff: Record<string, SchemaDiff>) {
	if (!diff) {
		return
	}

	let newSchema = structuredClone(schema)

	Object.keys(diff).forEach((key) => {
		if (diff[key].diff === 'removed') {
			delete newSchema.properties[key]
			if (newSchema.order) {
				newSchema.order = newSchema.order.filter((x) => x !== key)
			}
		} else if (diff[key].diff === 'added' || diff[key].diff === 'modified') {
			newSchema.properties[key] = diff[key].fullSchema
			if (newSchema.order && !newSchema.order.includes(key)) {
				newSchema.order.push(key)
			}
		} else if (typeof diff[key].diff === 'object') {
			newSchema.properties[key] = applyDiff(newSchema.properties[key], diff[key].diff)
		}
	})

	return newSchema
}
