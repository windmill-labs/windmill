/**
 * Component-Managed Fields Registry
 *
 * This module provides a centralized registry of fields that are automatically
 * managed by components at runtime, rather than being configured by users.
 *
 * These fields should never be treated as static fields in force_viewer_static_fields,
 * as their values are determined dynamically by user interactions (pagination,
 * sorting, search, chat input, etc.)
 */

import type { AppInput } from '$lib/components/apps/inputType'

/**
 * Registry of component types to their managed field names
 */
export const COMPONENT_MANAGED_FIELDS: Record<string, string[]> = {
	aggridinfinitecomponent: ['offset', 'limit', 'orderBy', 'isDesc', 'search'],
	aggridinfinitecomponentee: ['offset', 'limit', 'orderBy', 'isDesc', 'search'],
	chatcomponent: ['user_message']
}

/**
 * Get the list of managed fields for a given component type
 */
export function getManagedFields(componentType: string): string[] {
	return COMPONENT_MANAGED_FIELDS[componentType] ?? []
}

/**
 * Check if a field is managed by a specific component type
 */
export function isFieldManagedByComponent(componentType: string, fieldName: string): boolean {
	const managedFields = getManagedFields(componentType)
	return managedFields.includes(fieldName)
}

/**
 * Convert component-managed fields from static to evalv2 type
 * This ensures they are properly handled at runtime and not added to force_viewer_static_fields
 */
export function convertManagedFieldsToEvalv2(
	componentType: string,
	componentId: string,
	fields: Record<string, AppInput>
): Record<string, AppInput> {
	const managedFieldNames = getManagedFields(componentType)

	if (managedFieldNames.length === 0) {
		return fields
	}

	const convertedFields = { ...fields }

	for (const fieldName of managedFieldNames) {
		if (convertedFields[fieldName]) {
			// Determine the expression based on component type
			let expr: string

			if (
				componentType === 'aggridinfinitecomponent' ||
				componentType === 'aggridinfinitecomponentee'
			) {
				// AgGrid components use params.{fieldName}
				expr = `${componentId}.params.${fieldName}`
			} else if (componentType === 'chatcomponent') {
				// Chat component uses userMessage output
				expr = `${componentId}.userMessage`
			} else {
				// Default pattern for future components
				expr = `${componentId}.${fieldName}`
			}

			// Convert to evalv2 type while preserving fieldType
			convertedFields[fieldName] = {
				type: 'evalv2',
				expr,
				fieldType: convertedFields[fieldName].fieldType ?? 'string',
				connections: []
			} as AppInput
		}
	}

	return convertedFields
}

/**
 * Get all managed field names across all component types
 * Useful for generic checks
 */
export function getAllManagedFieldNames(): string[] {
	const allFields = new Set<string>()
	Object.values(COMPONENT_MANAGED_FIELDS).forEach((fields) => {
		fields.forEach((field) => allFields.add(field))
	})
	return Array.from(allFields)
}
