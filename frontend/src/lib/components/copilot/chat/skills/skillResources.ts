import { ResourceService } from '$lib/gen'
import { canWrite } from '$lib/utils'
import type { UserExt } from '$lib/stores'

/**
 * Skills are resources of this type: a file resource (`format_extension = 'md'`)
 * whose `value.content` is the SKILL.md body, whose description column is what the
 * assistant reads when deciding the skill applies, and whose path names it.
 */
export const SKILLS_RESOURCE_TYPE = 'ai_skill'

/** A skill as the picker and the system prompt see it — never the body, which
 * `read_skill` fetches only once the model commits to using the skill. */
export type SkillResource = {
	path: string
	/** Path basename: what the `/` command and the picker row show. */
	name: string
	description: string
	editedAt?: string
	canWrite: boolean
}

/** The `/`-command and display name for a skill. Paths are `[ufg]/x/y…`, so the
 * last segment is always present. */
export function skillNameFromPath(path: string): string {
	return path.split('/').pop() ?? path
}

/** Basenames carried by more than one of these skills. Two folders can each hold
 * a `deploy`, and then the name alone no longer says which one — the picker shows
 * the path for these, and the `/` command refuses to guess. */
export function ambiguousSkillNames(skills: readonly { name: string }[]): Set<string> {
	const seen = new Map<string, number>()
	for (const s of skills) seen.set(s.name, (seen.get(s.name) ?? 0) + 1)
	return new Set([...seen].filter(([, n]) => n > 1).map(([name]) => name))
}

const SKILLS_PAGE_SIZE = 100
/** Pages to walk before giving up. Ordinary resources and repeated imports can
 * make any number of skills, and a single page would drop the rest — including a
 * selected one, which would then vanish from the prompt with nothing to explain
 * it. The bound is a guard against a paging bug looping forever, not a product
 * cap, so reaching it is reported rather than passed off as the whole set. */
const MAX_SKILLS_PAGES = 100

/** The rows read, and whether the walk stopped at the bound rather than the end.
 * Reported rather than thrown: a truncated read is still most of the skills, and
 * dropping them all would take every selected skill out of the prompt at once. */
export type SkillListing = { skills: SkillResource[]; truncated: boolean }

/** Every skill resource readable in the workspace.
 *
 * `user` decides which rows the drawer offers to edit rather than only view; pass
 * the account the workspace is being browsed as. Ownership is mostly implicit in
 * the path (`u/<me>/…`, a folder the user owns), which is why this goes through
 * the shared `canWrite` rather than reading `extra_perms` alone. */
export async function listSkillResources(
	workspace: string,
	user?: UserExt
): Promise<SkillListing> {
	if (!workspace) return { skills: [], truncated: false }
	const rows: SkillResource[] = []
	for (let page = 1; page <= MAX_SKILLS_PAGES; page++) {
		const resources = await ResourceService.listResource({
			workspace,
			resourceType: SKILLS_RESOURCE_TYPE,
			page,
			perPage: SKILLS_PAGE_SIZE
		})
		rows.push(
			...resources.map((r) => ({
				path: r.path,
				name: skillNameFromPath(r.path),
				description: r.description ?? '',
				editedAt: r.edited_at,
				canWrite: canWrite(r.path, r.extra_perms ?? {}, user)
			}))
		)
		if (resources.length < SKILLS_PAGE_SIZE) return { skills: rows, truncated: false }
	}
	return { skills: rows, truncated: true }
}

/** Cut `text` to `maxChars` code points. For the description, whose cap is stated
 * in characters — cutting that one by bytes would reduce a legal 1,024-character
 * CJK description to about a third of itself. */
export function truncateChars(text: string, maxChars: number): string {
	const points = [...text]
	return points.length <= maxChars ? text : `${points.slice(0, maxChars).join('')}… [truncated]`
}

/** Cut `text` to `maxBytes` of UTF-8, marking the cut so a reader (the model
 * included) can tell truncation from a body that simply ends there.
 *
 * For the body, whose cap is a byte budget: 64k CJK characters are ~192 KiB, so a
 * code-unit cut would let three times the intended payload through. */
export function truncateForPrompt(text: string, maxBytes: number): string {
	const encoded = new TextEncoder().encode(text)
	if (encoded.byteLength <= maxBytes) return text
	// `fatal: false` replaces the partial code point a byte-aligned cut can leave
	// with U+FFFD; dropping it keeps the tail clean.
	const cut = new TextDecoder('utf-8').decode(encoded.slice(0, maxBytes)).replace(/�$/, '')
	return `${cut}… [truncated]`
}

/** The SKILL.md body of one skill. Throws rather than returning `''` when the
 * resource holds no readable body: an empty string reaches the model as a
 * successful read of a skill with no instructions, which it would then act on.
 *
 * Deliberately unbounded — the editor loads through here and saves what it loaded,
 * so truncating would rewrite an over-long skill the first time someone opened it.
 * Bounding belongs at the prompt boundary, where the cost actually is. */
export async function readSkillBody(workspace: string, path: string): Promise<string> {
	const value = (await ResourceService.getResourceValue({ workspace, path })) as
		| { content?: unknown }
		| undefined
	if (typeof value?.content !== 'string') {
		throw new Error(`resource ${path} has no string "content" — is it an ${SKILLS_RESOURCE_TYPE}?`)
	}
	return value.content
}

export async function saveSkillResource(
	workspace: string,
	path: string,
	description: string,
	instructions: string,
	{ overwrite = false }: { overwrite?: boolean } = {}
): Promise<void> {
	await ResourceService.createResource({
		workspace,
		updateIfExists: overwrite,
		requestBody: {
			path,
			description,
			value: { content: instructions },
			resource_type: SKILLS_RESOURCE_TYPE
		}
	})
}

/** Save an edit to an existing skill, moving it when the path changed. */
export async function updateSkillResource(
	workspace: string,
	currentPath: string,
	path: string,
	description: string,
	instructions: string
): Promise<void> {
	await ResourceService.updateResource({
		workspace,
		path: currentPath,
		requestBody: { path, description, value: { content: instructions } }
	})
}

export async function deleteSkillResource(workspace: string, path: string): Promise<void> {
	await ResourceService.deleteResource({ workspace, path })
}
