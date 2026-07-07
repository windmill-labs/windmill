import YAML from 'yaml'
import { z } from 'zod'

export type SkillUpload = { name: string; description: string; instructions: string }

// `name` + `description` mirror the Claude SKILL.md spec (counted in characters);
// the body is a byte-bounded payload. Keep these in sync with backend `validate_skill`.
export const MAX_SKILL_NAME_LENGTH = 64
export const MAX_SKILL_DESCRIPTION_LENGTH = 1_024
export const MAX_SKILL_INSTRUCTIONS_LENGTH = 64 * 1024

const textEncoder = new TextEncoder()

// Single source of truth for skill field validation, shared by the paste/edit
// modal and the folder importer. Lengths are code-point / byte bounded to match
// the backend, so `.refine` (not `.max`, which counts UTF-16 units) is used.
export const skillSchema = z.object({
	name: z
		.string()
		.min(1, 'name is required')
		.refine(
			(v) => [...v].length <= MAX_SKILL_NAME_LENGTH,
			`name is longer than ${MAX_SKILL_NAME_LENGTH} characters`
		)
		.refine((v) => /^[a-z0-9-]+$/.test(v), {
			error: (iss) =>
				`name ${JSON.stringify(iss.input)} must only contain lowercase letters, digits or '-'`
		}),
	description: z
		.string()
		.min(1, 'description is required')
		.refine(
			(v) => [...v].length <= MAX_SKILL_DESCRIPTION_LENGTH,
			`description is longer than ${MAX_SKILL_DESCRIPTION_LENGTH} characters`
		),
	instructions: z
		.string()
		.min(1, 'body is required')
		.refine(
			(v) => textEncoder.encode(v).byteLength <= MAX_SKILL_INSTRUCTIONS_LENGTH,
			`body is longer than ${MAX_SKILL_INSTRUCTIONS_LENGTH} bytes`
		)
})

/** First validation error for a skill, or `undefined` if it is valid. */
export function validateSkill(skill: SkillUpload): string | undefined {
	const result = skillSchema.safeParse(skill)
	return result.success ? undefined : result.error.issues[0]?.message
}

/** Split a SKILL.md into its frontmatter `name`/`description` and the markdown body. */
export function parseSkillMd(raw: string): {
	name: string | undefined
	description: string | undefined
	instructions: string
} {
	const text = raw.replace(/^﻿/, '')
	const fm = /^---\s*\r?\n([\s\S]*?)\r?\n---\s*\r?\n?/.exec(text)
	if (!fm) {
		return { name: undefined, description: undefined, instructions: text.trim() }
	}
	let name: string | undefined
	let description: string | undefined
	try {
		const data = YAML.parse(fm[1]) ?? {}
		if (typeof data?.name === 'string') name = data.name.trim()
		if (typeof data?.description === 'string') description = data.description.trim()
	} catch {
		// Malformed frontmatter — fall through so the skill is reported as invalid
		// rather than silently dropped.
	}
	return { name, description, instructions: text.slice(fm[0].length).trim() }
}

/**
 * Parse a SKILL.md and validate it in one step. `nameOverride` lets the folder
 * importer supply the skill name from its containing folder instead of the
 * frontmatter. Returns the validated skill or the first error message.
 */
export function parseAndValidateSkill(
	raw: string,
	nameOverride?: string
): { skill: SkillUpload } | { error: string } {
	const parsed = parseSkillMd(raw)
	const candidate: SkillUpload = {
		name: nameOverride ?? parsed.name ?? '',
		description: parsed.description ?? '',
		instructions: parsed.instructions
	}
	const error = validateSkill(candidate)
	return error ? { error } : { skill: candidate }
}

/** Reconstruct a SKILL.md from its stored parts for editing/rendering. */
export function buildSkillMd(skill: SkillUpload): string {
	const frontmatter = YAML.stringify({
		name: skill.name,
		description: skill.description
	}).trimEnd()
	return `---\n${frontmatter}\n---\n\n${skill.instructions}\n`
}
