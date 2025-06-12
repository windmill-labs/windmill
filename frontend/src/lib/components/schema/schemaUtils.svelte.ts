export type SchemaDiff = {
	diff: 'same' | 'added' | 'removed' | 'modified' | Record<string, SchemaDiff>
	fullSchema: { [key: string]: any } | undefined
	oldSchema?: { [key: string]: any } | undefined
}

function isCompatible(diff: Record<string, SchemaDiff>) {
	let compatible = true
	Object.values(diff).forEach((diff) => {
		if (diff.diff === 'added' || diff.diff === 'modified' || diff.diff === 'removed') {
			compatible = false
		} else if (isRecordSchemaDiff(diff.diff)) {
			compatible = isCompatible(diff.diff)
		}
	})
	return compatible
}

function isCompatibleObject(a: any, b: any): boolean {
	if (!a || !b) {
		return false
	}

	if (a.type === 'null' || b.type === 'null') {
		return true
	}

	if (a.type !== b.type) {
		if (
			(a.type === 'number' || a.type === 'integer') &&
			(b.type === 'number' || b.type === 'integer')
		) {
			return true
		}
		return false
	}

	switch (a.type) {
		case 'object':
			if (a.oneOf || b.oneOf) {
				//TODO: handle oneOf compatibility. here we assume that only b is oneOf
				return true
			}
			return a.format === b.format

		case 'array':
			return !a.items || !b.items || a.items?.type === b.items?.type

		case 'string':
			return a.format === b.format

		case 'boolean':
		case 'number':
		case 'integer':
			return true

		default:
			return false
	}
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
			} else {
				const previewProp = previewSchema.properties[key]
				const currentProp = currentSchema.properties[key]
				if (previewProp.type === 'object' && currentProp.type === 'object') {
					if (JSON.stringify(previewProp) === JSON.stringify(currentProp)) {
						diff[key] = { diff: 'same', fullSchema: undefined }
					} else if (previewProp.oneOf || currentProp.oneOf) {
						if (isCompatibleObject(previewProp, currentProp)) {
							diff[key] = { diff: 'same', fullSchema: undefined }
						} else {
							diff[key] = { diff: 'modified', fullSchema: previewProp, oldSchema: currentProp }
						}
					} else if (previewProp.format || currentProp.format) {
						//TODO: handle s3 object compatibility
						diff[key] = { diff: 'modified', fullSchema: previewProp, oldSchema: currentProp }
					} else if (!previewProp.properties || !currentProp.properties) {
						// Handle case where one of the object does not have properties
						diff[key] = { diff: 'modified', fullSchema: previewProp, oldSchema: currentProp }
					} else {
						const diffProp = computeDiff(previewProp, currentProp)
						const checkIfSame = isCompatible(diffProp)
						if (checkIfSame) {
							diff[key] = { diff: 'same', fullSchema: undefined }
						} else {
							diff[key] = {
								diff: diffProp,
								fullSchema: previewProp,
								oldSchema: currentProp
							}
						}
					}
				} else if (isCompatibleObject(previewProp, currentProp)) {
					diff[key] = { diff: 'same', fullSchema: undefined }
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
	const newSchema = structuredClone($state.snapshot(schema))
	Object.keys(diff).forEach((key) => {
		const diffValue = diff[key].diff
		if (diffValue === 'added' || diffValue === 'modified') {
			newSchema.properties[key] = diff[key].fullSchema
			if (newSchema.order && !newSchema.order.includes(key)) {
				newSchema.order.push(key)
			}
		} else if (isRecordSchemaDiff(diffValue)) {
			// Handle nested diffs
			newSchema.properties[key] = schemaFromDiff(diffValue, schema.properties[key])
		}
	})
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

function isRecordSchemaDiff(value: SchemaDiff['diff']): value is Record<string, SchemaDiff> {
	return typeof value === 'object' && value !== null
}

export function applyDiff(
	schema: Record<string, any> | undefined,
	diff: Record<string, SchemaDiff> | undefined
) {
	if (!diff || !schema) {
		return
	}

	let newSchema = structuredClone($state.snapshot(schema))

	Object.keys(diff).forEach((key) => {
		const diffValue = diff[key].diff
		if (diffValue === 'removed') {
			delete newSchema.properties[key]
			if (newSchema.order) {
				newSchema.order = newSchema.order.filter((x) => x !== key)
			}
		} else if (diffValue === 'added' || diffValue === 'modified' || isRecordSchemaDiff(diffValue)) {
			newSchema.properties[key] = diff[key].fullSchema
			if (newSchema.order && !newSchema.order.includes(key)) {
				newSchema.order.push(key)
			}
		}
	})

	return newSchema
}
