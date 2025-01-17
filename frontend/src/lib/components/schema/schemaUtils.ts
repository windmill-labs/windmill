export type SchemaDiff = {
	diff: 'same' | 'added' | 'removed' | 'modified' | Record<string, SchemaDiff>
	fullSchema: { [key: string]: any } | undefined
	oldSchema?: { [key: string]: any } | undefined
}

function filterUndefinedFields(obj: any): any {
	if (!obj || typeof obj !== 'object') return obj
	delete obj.order

	const filtered: any = {}
	Object.entries(obj).forEach(([key, value]) => {
		if (value !== undefined) {
			if (key === 'additionalProperties' && value === false) {
				delete filtered.additionalProperties
			} else {
				filtered[key] = typeof value === 'object' ? filterUndefinedFields(value) : value
			}
		} else {
			delete filtered[key]
		}
	})
	return filtered
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
				const filteredPreviewProp = filterUndefinedFields({ ...previewProp })
				const filteredCurrentProp = filterUndefinedFields({ ...currentProp })
				if (JSON.stringify(filteredPreviewProp) === JSON.stringify(filteredCurrentProp)) {
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
	const newSchema = structuredClone(schema)
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

	let newSchema = structuredClone(schema)

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
