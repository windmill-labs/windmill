import { ResourceService } from '$lib/gen'

/**
 * Resource types are named by abbreviation — `gdrive`, `gcal`, `s3` — so a product's real
 * name ("Google Drive", "Amazon S3") only ever appears in its description. Matching on the
 * name alone means searching "google" finds none of the Google integrations.
 */
export function resourceTypeSearchText(name: string, description?: string): string {
	return description ? `${name} ${description}` : name
}

export async function loadResourceTypeSearchText(
	workspace: string
): Promise<Record<string, string>> {
	const types = await ResourceService.listResourceType({ workspace })
	return Object.fromEntries(
		types.map((t) => [t.name, resourceTypeSearchText(t.name, t.description)])
	)
}

/** Human-facing label for a resource type: `git_repository` -> `Git repository`. */
export function resourceTypeLabel(name: string): string {
	const spaced = name.replace(/_/g, ' ')
	return spaced.charAt(0).toUpperCase() + spaced.slice(1)
}

/** "Add **a** Supabase resource" / "Add **an** Airtable resource". */
export function resourceTypeArticle(name: string): string {
	return /^[aeiou]/i.test(name) ? 'an' : 'a'
}

/** Drawer title for creating a resource, named after its type once one is picked. */
export function addResourceTitle(resourceType: string | undefined): string {
	return resourceType
		? `Add ${resourceTypeArticle(resourceType)} ${resourceTypeLabel(resourceType)} resource`
		: 'Add a resource'
}
