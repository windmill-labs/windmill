interface DelegateToGitRepoConfig {
	resource?: string
	playbook?: string
	inventories_location?: string
}

/**
 * Updates a specific field in the delegate_to_git_repo section
 * @param code - The current YAML script content
 * @param fieldName - The field name to update (resource, playbook, inventories_location)
 * @param value - The value to set (or undefined to remove the field)
 * @returns The modified YAML script content
 */
export function updateDelegateToGitRepoField(code: string, fieldName: string, value: string | undefined): string {
	const lines = code.split('\n')

	// Find delegate_to_git_repo section
	const delegateLineIndex = lines.findIndex(line => line.trim().startsWith('delegate_to_git_repo:'))

	if (delegateLineIndex === -1) {
		// If no delegate section exists and we're setting a value, create the whole section
		if (value !== undefined) {
			return insertDelegateToGitRepoSection(code, { [fieldName]: value })
		}
		return code
	}

	// Find the specific field line
	const fieldLineIndex = lines.findIndex((line, index) =>
		index > delegateLineIndex && line.trim().startsWith(`${fieldName}:`)
	)

	if (fieldLineIndex !== -1) {
		if (value !== undefined) {
			// Update existing field
			lines[fieldLineIndex] = `  ${fieldName}: ${value}`
		} else {
			// Remove field
			lines.splice(fieldLineIndex, 1)
		}
	} else if (value !== undefined) {
		// Add new field after delegate_to_git_repo line
		lines.splice(delegateLineIndex + 1, 0, `  ${fieldName}: ${value}`)
	}

	return lines.join('\n')
}

/**
 * Inserts or updates multiple fields in a delegate_to_git_repo section
 * @param code - The current YAML script content
 * @param config - Configuration object with fields to update
 * @returns The modified YAML script content
 */
export function updateDelegateToGitRepoConfig(code: string, config: DelegateToGitRepoConfig): string {
	let updatedCode = code

	// Update each field that's provided
	for (const [fieldName, value] of Object.entries(config)) {
		if (value !== undefined) {
			updatedCode = updateDelegateToGitRepoField(updatedCode, fieldName, value)
		}
	}

	return updatedCode
}

/**
 * Legacy function for backward compatibility
 * Inserts or updates a delegate_to_git_repo section in an Ansible YAML script
 * @param code - The current YAML script content
 * @param resourcePath - The git repository resource path to delegate to
 * @returns The modified YAML script content
 */
export function insertDelegateToGitRepoInCode(code: string, resourcePath: string): string {
	return updateDelegateToGitRepoField(code, 'resource', resourcePath)
}

/**
 * Inserts a new delegate_to_git_repo section with the given configuration
 * @param code - The current YAML script content
 * @param config - Configuration object with fields to set
 * @returns The modified YAML script content
 */
function insertDelegateToGitRepoSection(code: string, config: DelegateToGitRepoConfig): string {
	const lines = code.split('\n')

	// Build the delegate section with all provided fields
	const delegateSection = ['delegate_to_git_repo:']

	// Add fields in a consistent order
	if (config.resource) {
		delegateSection.push(`  resource: ${config.resource}`)
	}
	if (config.playbook) {
		delegateSection.push(`  playbook: ${config.playbook}`)
	}
	if (config.inventories_location) {
		delegateSection.push(`  inventories_location: ${config.inventories_location}`)
	}

	// Find a good insertion point (after ---, then after inventories if they exist, otherwise at the top)
	let insertionIndex = 0

	// First, skip whitespace and find document start marker ---
	for (let i = 0; i < lines.length; i++) {
		const trimmedLine = lines[i].trim()
		if (trimmedLine === '---') {
			insertionIndex = i + 1 // Start after the document marker
			break
		} else if (trimmedLine && !trimmedLine.startsWith('#')) {
			// Hit non-comment, non-whitespace content without finding ---, stop looking
			break
		}
	}

	// Look for the end of inventories section
	for (let i = insertionIndex; i < lines.length; i++) {
		const line = lines[i].trim()
		if (line.startsWith('inventories:')) {
			// Find the end of inventories section
			for (let j = i + 1; j < lines.length; j++) {
				const nextLine = lines[j].trim()
				if (nextLine && !nextLine.startsWith('-') && !nextLine.startsWith(' ') && !nextLine.startsWith('#')) {
					insertionIndex = j
					break
				}
			}
			break
		} else if (line && !line.startsWith('#') && insertionIndex <= 1) {
			// First non-comment line after ---, insert before it
			insertionIndex = i
			break
		}
	}

	// Insert the delegate section
	lines.splice(insertionIndex, 0, ...delegateSection, '')

	return lines.join('\n')
}

/**
 * Generic function to extract a specific field from delegate_to_git_repo section
 * @param code - The YAML script content
 * @param fieldName - The field name to extract (resource, playbook, inventories_location)
 * @returns The field value if found, undefined otherwise
 */
function extractDelegateToGitRepoField(code: string, fieldName: string): string | undefined {
	const lines = code.split('\n')

	// Find delegate_to_git_repo section
	const delegateLineIndex = lines.findIndex(line => line.trim().startsWith('delegate_to_git_repo:'))

	if (delegateLineIndex === -1) {
		return undefined
	}

	// Look for the field line after delegate_to_git_repo
	for (let i = delegateLineIndex + 1; i < lines.length; i++) {
		const line = lines[i].trim()
		if (line.startsWith(`${fieldName}:`)) {
			// Extract the field value (everything after "fieldName:")
			const fieldMatch = line.match(new RegExp(`^${fieldName}:\\s*(.+)$`))
			return fieldMatch?.[1]?.trim()
		} else if (line && !line.startsWith(' ') && !line.startsWith('\t')) {
			// Hit a new top-level section, stop looking
			break
		}
	}

	return undefined
}

/**
 * Extracts the current git repository resource from delegate_to_git_repo section
 * @param code - The YAML script content
 * @returns The resource path if found, undefined otherwise
 */
export function extractCurrentGitRepoResource(code: string): string | undefined {
	return extractDelegateToGitRepoField(code, 'resource')
}

/**
 * Extracts the current playbook path from delegate_to_git_repo section
 * @param code - The YAML script content
 * @returns The playbook path if found, undefined otherwise
 */
export function extractDelegateToGitRepoPlaybook(code: string): string | undefined {
	return extractDelegateToGitRepoField(code, 'playbook')
}

/**
 * Extracts the current inventories location from delegate_to_git_repo section
 * @param code - The YAML script content
 * @returns The inventories location if found, undefined otherwise
 */
export function extractDelegateToGitRepoInventoriesLocation(code: string): string | undefined {
	return extractDelegateToGitRepoField(code, 'inventories_location')
}

/**
 * Extracts all delegate_to_git_repo configuration from the code
 * @param code - The YAML script content
 * @returns Configuration object with all extracted fields
 */
export function extractDelegateToGitRepoConfig(code: string): DelegateToGitRepoConfig {
	return {
		resource: extractCurrentGitRepoResource(code),
		playbook: extractDelegateToGitRepoPlaybook(code),
		inventories_location: extractDelegateToGitRepoInventoriesLocation(code)
	}
}

/**
 * Inserts or updates additional_inventories section in an Ansible YAML script
 * @param code - The current YAML script content
 * @param inventoryPaths - Array of inventory file paths
 * @returns The modified YAML script content
 */
export function insertAdditionalInventories(code: string, inventoryPaths: string[]): string {
	const lines = code.split('\n')

	// Find and update existing additional_inventories section if it exists
	const additionalInventoriesIndex = lines.findIndex(line => line.trim().startsWith('additional_inventories:'))
	if (additionalInventoriesIndex !== -1) {
		// Determine the indentation level of the additional_inventories line
		const sectionLine = lines[additionalInventoriesIndex]
		const sectionIndentation = sectionLine.length - sectionLine.trimStart().length

		// Find the options: field within the section
		let optionsIndex = -1
		let optionsEndIndex = -1

		for (let i = additionalInventoriesIndex + 1; i < lines.length; i++) {
			const line = lines[i]
			const trimmedLine = line.trim()

			// Skip empty lines
			if (!trimmedLine) {
				continue
			}

			// Calculate indentation of current line
			const currentIndentation = line.length - line.trimStart().length

			// If we find a line with same or lesser indentation than the section header,
			// we've reached the end of the additional_inventories section
			if (currentIndentation <= sectionIndentation) {
				break
			}

			// Look for options: field (should be directly under additional_inventories)
			if (trimmedLine.startsWith('- options:') && currentIndentation > sectionIndentation) {
				optionsIndex = i

				// Check if it's inline format: options: [...]
				if (trimmedLine.includes('[') && trimmedLine.includes(']')) {
					// Inline format - just this line
					optionsEndIndex = i + 1
					break
				} else {
					// Dash format - find all the dash items
					optionsEndIndex = i + 1
					for (let j = i + 1; j < lines.length; j++) {
						const nextLine = lines[j]
						const nextTrimmed = nextLine.trim()
						const nextIndentation = nextLine.length - nextLine.trimStart().length

						// Skip empty lines
						if (!nextTrimmed) {
							continue
						}

						// If we hit a line that's not more indented than options:, we're done
						if (nextIndentation <= currentIndentation) {
							break
						}

						// If it's a dash item, include it
						if (nextTrimmed.startsWith('-')) {
							optionsEndIndex = j + 1
						} else {
							// Hit a non-dash line that's indented - stop here
							break
						}
					}
					break
				}
			}
		}

		// Format the new options content
		const optionsIndentation = '  ' // Standard 2-space indentation under additional_inventories
		const formattedPaths = inventoryPaths.map(path => `"delegated_git_repository/${path}"`)
		const inlineFormat = `${optionsIndentation}- options: [${formattedPaths.join(', ')}]`

		let newOptionsContent: string[]
		if (inlineFormat.length <= 100) {
			// Use inline format
			newOptionsContent = [inlineFormat]
		} else {
			// Use dash format
			newOptionsContent = [`${optionsIndentation}- options:`]
			inventoryPaths.forEach(path => {
				newOptionsContent.push(`${optionsIndentation}  - "delegated_git_repository/${path}"`)
			})
		}

		if (optionsIndex !== -1) {
			// Replace existing options: field
			lines.splice(optionsIndex, optionsEndIndex - optionsIndex, ...newOptionsContent)
		} else {
			// Add options: field to existing section (right after the section header)
			lines.splice(additionalInventoriesIndex + 1, 0, ...newOptionsContent)
		}

		return lines.join('\n')
	}

	// Format the inventory paths based on length
	const formattedPaths = inventoryPaths.map(path => `"delegated_git_repository/${path}"`)
	const inlineFormat = `options: [${formattedPaths.join(', ')}]`

	let inventorySection: string[]
	if (inlineFormat.length <= 100) {
		// Use inline format
		inventorySection = [
			'additional_inventories:',
			`  ${inlineFormat}`
		]
	} else {
		// Use dash format with each item on new line
		inventorySection = [
			'additional_inventories:',
			'  - options:'
		]
		inventoryPaths.forEach(path => {
			inventorySection.push(`    - "delegated_git_repository/${path}"`)
		})
	}

	// Find insertion point (after the complete delegate_to_git_repo section)
	const delegateLineIndex = lines.findIndex(line => line.trim().startsWith('delegate_to_git_repo:'))
	if (delegateLineIndex === -1) {
		// If no delegate_to_git_repo section, insert at the beginning (after document marker if exists)
		let insertionIndex = 0
		for (let i = 0; i < lines.length; i++) {
			const trimmedLine = lines[i].trim()
			if (trimmedLine === '---') {
				insertionIndex = i + 1
				break
			} else if (trimmedLine && !trimmedLine.startsWith('#')) {
				break
			}
		}
		lines.splice(insertionIndex, 0, ...inventorySection, '')
	} else {
		// Find the last actual content line of the delegate_to_git_repo section
		let lastContentIndex = delegateLineIndex
		for (let i = delegateLineIndex + 1; i < lines.length; i++) {
			const line = lines[i]
			const trimmedLine = line.trim()

			// If we hit a non-empty line that doesn't start with whitespace (not indented),
			// we've reached the end of the delegate_to_git_repo section
			if (trimmedLine && !line.startsWith(' ') && !line.startsWith('\t')) {
				break
			}

			// If this is an indented non-empty line, it's part of delegate_to_git_repo
			if (trimmedLine && (line.startsWith(' ') || line.startsWith('\t'))) {
				lastContentIndex = i
			}
		}

		// Insert right after the last content line of delegate_to_git_repo
		const insertionIndex = lastContentIndex + 1
		lines.splice(insertionIndex, 0, ...inventorySection, '')
	}

	return lines.join('\n')
}

