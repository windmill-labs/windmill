import { describe, expect, it } from 'vitest'

import {
	MAX_SKILL_DESCRIPTION_LENGTH,
	MAX_SKILL_INSTRUCTIONS_LENGTH,
	MAX_SKILL_NAME_LENGTH,
	buildSkillMd,
	parseAndValidateSkill,
	parseSkillMd,
	validateSkill
} from './aiSkills'

describe('parseSkillMd', () => {
	it('splits frontmatter name/description from the body', () => {
		const md = '---\nname: my-skill\ndescription: does a thing\n---\n\n# Title\n\nBody text.'
		expect(parseSkillMd(md)).toEqual({
			name: 'my-skill',
			description: 'does a thing',
			instructions: '# Title\n\nBody text.'
		})
	})

	it('trims frontmatter values and the body', () => {
		const md = '---\nname:   spaced  \ndescription:  padded \n---\n\n  body  '
		const parsed = parseSkillMd(md)
		expect(parsed.name).toBe('spaced')
		expect(parsed.description).toBe('padded')
		expect(parsed.instructions).toBe('body')
	})

	it('returns undefined name/description when there is no frontmatter', () => {
		expect(parseSkillMd('just a body')).toEqual({
			name: undefined,
			description: undefined,
			instructions: 'just a body'
		})
	})

	it('strips a leading UTF-8 BOM before matching frontmatter', () => {
		const md = '﻿---\nname: bom-skill\ndescription: d\n---\n\nbody'
		const parsed = parseSkillMd(md)
		expect(parsed.name).toBe('bom-skill')
		expect(parsed.description).toBe('d')
		expect(parsed.instructions).toBe('body')
	})

	it('handles CRLF line endings in the frontmatter fence', () => {
		const md = '---\r\nname: crlf\r\ndescription: d\r\n---\r\n\r\nbody'
		const parsed = parseSkillMd(md)
		expect(parsed.name).toBe('crlf')
		expect(parsed.description).toBe('d')
	})

	it('leaves name/description undefined for missing frontmatter keys', () => {
		const parsed = parseSkillMd('---\nname: only-name\n---\n\nbody')
		expect(parsed.name).toBe('only-name')
		expect(parsed.description).toBeUndefined()
		expect(parsed.instructions).toBe('body')
	})

	it('ignores non-string frontmatter values', () => {
		const parsed = parseSkillMd('---\nname: 123\ndescription: [a, b]\n---\n\nbody')
		expect(parsed.name).toBeUndefined()
		expect(parsed.description).toBeUndefined()
	})

	it('does not throw on malformed YAML frontmatter', () => {
		const parsed = parseSkillMd('---\nname: "unterminated\n---\n\nbody')
		expect(parsed.instructions).toBe('body')
		expect(parsed.name).toBeUndefined()
	})
})

describe('validateSkill', () => {
	const valid = { name: 'a-skill', description: 'a description', instructions: 'body' }

	it('returns undefined for a valid skill', () => {
		expect(validateSkill(valid)).toBeUndefined()
	})

	it('flags a missing name', () => {
		expect(validateSkill({ ...valid, name: '' })).toBe('name is required')
	})

	it('flags a missing description', () => {
		expect(validateSkill({ ...valid, description: '' })).toBe('description is required')
	})

	it('flags a missing body', () => {
		expect(validateSkill({ ...valid, instructions: '' })).toBe('body is required')
	})

	it('rejects names with disallowed characters and echoes the value', () => {
		expect(validateSkill({ ...valid, name: 'Bad Name!' })).toBe(
			`name "Bad Name!" must only contain lowercase letters, digits or '-'`
		)
	})

	it('accepts names of lowercase letters, digits and hyphens', () => {
		expect(validateSkill({ ...valid, name: 'skill-123' })).toBeUndefined()
	})

	it('accepts a name exactly at the length limit but rejects one over', () => {
		expect(validateSkill({ ...valid, name: 'a'.repeat(MAX_SKILL_NAME_LENGTH) })).toBeUndefined()
		expect(validateSkill({ ...valid, name: 'a'.repeat(MAX_SKILL_NAME_LENGTH + 1) })).toBe(
			`name is longer than ${MAX_SKILL_NAME_LENGTH} characters`
		)
	})

	it('counts the description limit in code points, not UTF-16 units', () => {
		// Astral emoji are 2 UTF-16 units but 1 code point each.
		const desc = '😀'.repeat(MAX_SKILL_DESCRIPTION_LENGTH)
		expect(validateSkill({ ...valid, description: desc })).toBeUndefined()
		expect(validateSkill({ ...valid, description: desc + '😀' })).toBe(
			`description is longer than ${MAX_SKILL_DESCRIPTION_LENGTH} characters`
		)
	})

	it('counts the body limit in bytes', () => {
		// A 4-byte emoji fills the byte budget four times faster than its char count.
		const bodyAtLimit = 'a'.repeat(MAX_SKILL_INSTRUCTIONS_LENGTH)
		expect(validateSkill({ ...valid, instructions: bodyAtLimit })).toBeUndefined()
		expect(validateSkill({ ...valid, instructions: bodyAtLimit + 'a' })).toBe(
			`body is longer than ${MAX_SKILL_INSTRUCTIONS_LENGTH} bytes`
		)
		const multibyte = '😀'.repeat(MAX_SKILL_INSTRUCTIONS_LENGTH / 4 + 1)
		expect(validateSkill({ ...valid, instructions: multibyte })).toBe(
			`body is longer than ${MAX_SKILL_INSTRUCTIONS_LENGTH} bytes`
		)
	})

	it('reports the name issue first when several fields are invalid', () => {
		expect(validateSkill({ name: '', description: '', instructions: '' })).toBe('name is required')
	})
})

describe('parseAndValidateSkill', () => {
	it('parses and validates a well-formed SKILL.md', () => {
		const md = '---\nname: good-skill\ndescription: a good one\n---\n\nbody'
		expect(parseAndValidateSkill(md)).toEqual({
			skill: { name: 'good-skill', description: 'a good one', instructions: 'body' }
		})
	})

	it('uses nameOverride instead of the frontmatter name', () => {
		const md = '---\nname: frontmatter-name\ndescription: d\n---\n\nbody'
		const result = parseAndValidateSkill(md, 'folder-name')
		expect(result).toEqual({
			skill: { name: 'folder-name', description: 'd', instructions: 'body' }
		})
	})

	it('validates the nameOverride, not the frontmatter name', () => {
		const md = '---\nname: valid-frontmatter\ndescription: d\n---\n\nbody'
		expect(parseAndValidateSkill(md, 'Bad Folder!')).toEqual({
			error: `name "Bad Folder!" must only contain lowercase letters, digits or '-'`
		})
	})

	it('does not fall back to the frontmatter name for an empty override', () => {
		const md = '---\nname: has-name\ndescription: d\n---\n\nbody'
		expect(parseAndValidateSkill(md, '')).toEqual({ error: 'name is required' })
	})

	it('returns an error for a missing description', () => {
		expect(parseAndValidateSkill('---\nname: s\n---\n\nbody')).toEqual({
			error: 'description is required'
		})
	})

	it('returns an error for an empty body', () => {
		expect(parseAndValidateSkill('---\nname: s\ndescription: d\n---\n\n')).toEqual({
			error: 'body is required'
		})
	})
})

describe('buildSkillMd', () => {
	it('round-trips a skill through parseSkillMd', () => {
		const skill = { name: 'round-trip', description: 'desc', instructions: '# Body\n\ntext' }
		const parsed = parseSkillMd(buildSkillMd(skill))
		expect(parsed.name).toBe(skill.name)
		expect(parsed.description).toBe(skill.description)
		expect(parsed.instructions).toBe(skill.instructions)
	})

	it('quotes descriptions with YAML-special characters so they round-trip', () => {
		const skill = {
			name: 'colon-desc',
			description: 'value: with colon, #hash and : more',
			instructions: 'body'
		}
		const parsed = parseSkillMd(buildSkillMd(skill))
		expect(parsed.description).toBe(skill.description)
		expect(validateSkill(skill)).toBeUndefined()
	})
})
