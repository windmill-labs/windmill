/**
 * Utility functions for Ansible script manipulation
 */

/**
 * Inserts or updates a delegate_to_git_repo section in an Ansible YAML script
 * @param code - The current YAML script content
 * @param resourcePath - The git repository resource path to delegate to
 * @returns The modified YAML script content
 */
export function insertDelegateToGitRepoInCode(code: string, resourcePath: string): string {
	const lines = code.split('\n')
	
	// Check if delegate_to_git_repo already exists
	const delegateLineIndex = lines.findIndex(line => line.trim().startsWith('delegate_to_git_repo:'))
	
	if (delegateLineIndex !== -1) {
		// Update existing delegate_to_git_repo resource
		const resourceLineIndex = lines.findIndex((line, index) => 
			index > delegateLineIndex && line.trim().startsWith('resource:')
		)
		
		if (resourceLineIndex !== -1) {
			// Replace existing resource line
			lines[resourceLineIndex] = `  resource: ${resourcePath}`
		} else {
			// Add resource line after delegate_to_git_repo
			lines.splice(delegateLineIndex + 1, 0, `  resource: ${resourcePath}`)
		}
	} else {
		// Insert new delegate_to_git_repo section
		const delegateSection = [
			'delegate_to_git_repo:',
			`  resource: ${resourcePath}`
		]
		
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
	}
	
	return lines.join('\n')
}

/**
 * Extracts the current git repository resource from delegate_to_git_repo section
 * @param code - The YAML script content
 * @returns The resource path if found, undefined otherwise
 */
export function extractCurrentGitRepoResource(code: string): string | undefined {
	const lines = code.split('\n')
	
	// Find delegate_to_git_repo section
	const delegateLineIndex = lines.findIndex(line => line.trim().startsWith('delegate_to_git_repo:'))
	
	if (delegateLineIndex === -1) {
		return undefined
	}
	
	// Look for resource line after delegate_to_git_repo
	for (let i = delegateLineIndex + 1; i < lines.length; i++) {
		const line = lines[i].trim()
		if (line.startsWith('resource:')) {
			// Extract the resource path (everything after "resource:")
			const resourceMatch = line.match(/^resource:\s*(.+)$/)
			return resourceMatch?.[1]?.trim()
		} else if (line && !line.startsWith(' ') && !line.startsWith('\t')) {
			// Hit a new top-level section, stop looking
			break
		}
	}
	
	return undefined
}