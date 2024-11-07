import {
	type ObjectOption,
	isObjectOptionArray,
	isStringArray
} from '$lib/components/multiselect/types'
import type { StaticAppInput } from '../../inputType'

export function parseConfigOptions(resolvedConfigItems: StaticAppInput) {
	if (!resolvedConfigItems) {
		return []
	}
	if (isObjectOptionArray(resolvedConfigItems)) {
		return parseLabeledItems(resolvedConfigItems)
	} else if (isStringArray(resolvedConfigItems)) {
		return parseStringItems(resolvedConfigItems)
	}
	return []
}
export function parseLabeledItems(resolvedConfigItems: ObjectOption[]) {
	return resolvedConfigItems?.map((item: ObjectOption) => {
		if (!item || typeof item !== 'object') {
			console.error(
				'When labeled, MultiSelect component items should be an array of { label: string, value: string }.'
			)
			return {
				label: 'not object',
				value: 'not object'
			}
		}
		return {
			label: item?.label ?? 'undefined',
			value:
				typeof item?.value === 'object' ? JSON.stringify(item.value) : item?.value ?? 'undefined'
		}
	})
}
export function parseStringItems(resolvedConfigItems: string[]): ObjectOption[] {
	return resolvedConfigItems?.map((option: string) => {
		if (option === null || option === undefined || typeof option !== 'string') {
			console.error('When not labeled, MultiSelect component items should be an array of strings.')
			return { label: 'not string', value: 'not string' }
		}
		if (option === '') {
			return { label: 'empty string', value: 'empty string' }
		}
		return { label: option, value: option }
	})
}
