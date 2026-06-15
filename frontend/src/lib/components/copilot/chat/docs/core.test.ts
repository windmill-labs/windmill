import { describe, expect, it } from 'vitest'
import {
	buildDocsOutline,
	canonicalDocsPageUrl,
	extractDocsSection,
	normalizeDocsUrl,
	parseDocsHeadings,
	renderDocsPageResult,
	sanitizeDocsMarkdownLinks
} from './core'

const SAMPLE = `# Jobs

Intro text about jobs.

## Job kinds

Some kinds.

## Result

### Result of jobs that failed

\`\`\`
{ "error": "boom" }
\`\`\`

### Result streaming

#### Returning a stream directly

\`\`\`python
# Returning a stream directly is a comment heading that must be ignored
def main():
    pass
\`\`\`

## Retention policy

Final section.
`

describe('parseDocsHeadings', () => {
	it('parses headings with their levels and ignores headings inside fenced code blocks', () => {
		const headings = parseDocsHeadings(SAMPLE)
		const titles = headings.map((h) => `${h.level}:${h.title}`)

		expect(titles).toEqual([
			'1:Jobs',
			'2:Job kinds',
			'2:Result',
			'3:Result of jobs that failed',
			'3:Result streaming',
			'4:Returning a stream directly',
			'2:Retention policy'
		])
		// The "# Returning a stream directly is a comment..." line inside the
		// python fence must not be parsed as a heading.
		expect(titles).not.toContain('1:Returning a stream directly is a comment heading that must be ignored')
	})

	it('returns startIndex offsets that point at the heading line', () => {
		const headings = parseDocsHeadings(SAMPLE)
		for (const heading of headings) {
			expect(SAMPLE.slice(heading.startIndex)).toMatch(
				new RegExp(`^#{${heading.level}}\\s+${heading.title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`)
			)
		}
	})

	it('handles tilde fences', () => {
		const content = '# Title\n\n~~~\n# not a heading\n~~~\n\n## Real\n'
		const headings = parseDocsHeadings(content)
		expect(headings.map((h) => h.title)).toEqual(['Title', 'Real'])
	})
})

describe('extractDocsSection', () => {
	it('extracts a section from its heading up to the next same-or-higher level heading', () => {
		const section = extractDocsSection(SAMPLE, 'Result')
		expect(section).toBeDefined()
		expect(section).toContain('## Result')
		expect(section).toContain('### Result of jobs that failed')
		expect(section).toContain('### Result streaming')
		// Stops before the next level-2 heading.
		expect(section).not.toContain('## Retention policy')
	})

	it('matches case-insensitively and tolerates punctuation differences', () => {
		const section = extractDocsSection(SAMPLE, 'retention-policy!')
		expect(section).toBeDefined()
		expect(section).toContain('## Retention policy')
		expect(section).toContain('Final section.')
	})

	it('returns the deepest section bounded by the next same-level heading', () => {
		const section = extractDocsSection(SAMPLE, 'Result streaming')
		expect(section).toBeDefined()
		expect(section).toContain('### Result streaming')
		expect(section).toContain('#### Returning a stream directly')
		expect(section).not.toContain('## Retention policy')
	})

	it('returns undefined when no heading matches', () => {
		expect(extractDocsSection(SAMPLE, 'Nonexistent section')).toBeUndefined()
	})
})

describe('buildDocsOutline', () => {
	it('lists headings with approximate per-section sizes and indentation', () => {
		const outline = buildDocsOutline(SAMPLE)
		expect(outline).toContain('- Jobs (~')
		expect(outline).toContain('  - Job kinds (~')
		expect(outline).toContain('    - Result of jobs that failed (~')
	})

	it('handles pages with no headings', () => {
		expect(buildDocsOutline('just some text\nwith no headings')).toBe(
			'(no markdown headings found on this page)'
		)
	})
})

describe('normalizeDocsUrl', () => {
	it('appends .md to a bare path', () => {
		expect(normalizeDocsUrl('/docs/core_concepts/jobs')).toBe(
			'https://www.windmill.dev/docs/core_concepts/jobs.md'
		)
	})

	it('accepts a path without a leading slash', () => {
		expect(normalizeDocsUrl('docs/core_concepts/jobs')).toBe(
			'https://www.windmill.dev/docs/core_concepts/jobs.md'
		)
	})

	it('accepts a full URL and strips anchors and query strings', () => {
		expect(
			normalizeDocsUrl('https://www.windmill.dev/docs/core_concepts/jobs#result?foo=bar')
		).toBe('https://www.windmill.dev/docs/core_concepts/jobs.md')
	})

	it('does not double-append .md', () => {
		expect(normalizeDocsUrl('/docs/core_concepts/jobs.md')).toBe(
			'https://www.windmill.dev/docs/core_concepts/jobs.md'
		)
	})

	it('strips a trailing slash before appending .md', () => {
		expect(normalizeDocsUrl('/docs/core_concepts/jobs/')).toBe(
			'https://www.windmill.dev/docs/core_concepts/jobs.md'
		)
	})

	it('strips docusaurus numeric ordering prefixes from path segments', () => {
		expect(normalizeDocsUrl('/docs/flows/13_flow_branches')).toBe(
			'https://www.windmill.dev/docs/flows/flow_branches.md'
		)
	})

	it('converts a .mdx source suffix to .md', () => {
		expect(normalizeDocsUrl('/docs/flows/13_flow_branches.mdx')).toBe(
			'https://www.windmill.dev/docs/flows/flow_branches.md'
		)
	})
})

describe('sanitizeDocsMarkdownLinks', () => {
	const PAGE = 'https://www.windmill.dev/docs/flows/flow_editor.md'

	it('rewrites a relative .mdx source link to a canonical published URL', () => {
		expect(sanitizeDocsMarkdownLinks('See [retries](./14_retries.mdx) for more.', PAGE)).toBe(
			'See [retries](https://www.windmill.dev/docs/flows/retries) for more.'
		)
	})

	it('strips numeric prefixes from same-directory links', () => {
		expect(sanitizeDocsMarkdownLinks('[handling](./8_error_handling.mdx)', PAGE)).toBe(
			'[handling](https://www.windmill.dev/docs/flows/error_handling)'
		)
	})

	it('preserves anchors when rewriting', () => {
		expect(sanitizeDocsMarkdownLinks('[branch all](./13_flow_branches.mdx#branch-all)', PAGE)).toBe(
			'[branch all](https://www.windmill.dev/docs/flows/flow_branches#branch-all)'
		)
	})

	it('leaves image and external links untouched', () => {
		const input =
			'![diagram](./assets/flow_example.png) and [site](https://example.com/page.md)'
		expect(sanitizeDocsMarkdownLinks(input, PAGE)).toBe(input)
	})

	it('leaves bare anchor links untouched', () => {
		expect(sanitizeDocsMarkdownLinks('[top](#introduction)', PAGE)).toBe('[top](#introduction)')
	})

	// `../` links are authored against the docusaurus source tree, whose directory
	// depth differs from the published URL on slug-flattened pages, so resolving
	// them against the page URL is unreliable (a single `../` can over-escape just
	// as a double one does). All `../` links are left untouched and disambiguated
	// by the canonical "Source page" header instead.
	it('leaves single ../ cross-directory links untouched', () => {
		const input = '[handling](../core_concepts/8_error_handling.mdx)'
		expect(sanitizeDocsMarkdownLinks(input, PAGE)).toBe(input)
	})

	it('leaves double ../../ cross-directory links untouched', () => {
		const input = '[retries](../../flows/14_retries.md)'
		expect(sanitizeDocsMarkdownLinks(input, PAGE)).toBe(input)
	})
})

describe('canonicalDocsPageUrl', () => {
	it('returns the published URL without the .md suffix', () => {
		expect(canonicalDocsPageUrl('/docs/flows/flow_editor')).toBe(
			'https://www.windmill.dev/docs/flows/flow_editor'
		)
	})

	it('strips numeric prefixes so a source-style path maps to the published URL', () => {
		expect(canonicalDocsPageUrl('/docs/flows/14_retries.md')).toBe(
			'https://www.windmill.dev/docs/flows/retries'
		)
	})
})

describe('renderDocsPageResult', () => {
	it('returns the whole page when small and no section requested', () => {
		expect(renderDocsPageResult(SAMPLE)).toBe(SAMPLE)
	})

	it('returns an outline for large pages with no section requested', () => {
		const large = `# Big\n\n${'x'.repeat(25_000)}\n\n## Tail\n\nmore`
		const result = renderDocsPageResult(large)
		expect(result).toContain('This documentation page is large')
		expect(result).toContain('- Big (~')
		expect(result).toContain('- Tail (~')
	})

	it('returns the requested section content when found', () => {
		const result = renderDocsPageResult(SAMPLE, 'Job kinds')
		expect(result).toContain('## Job kinds')
		expect(result).toContain('Some kinds.')
	})

	it('returns the outline with a note when the requested section is missing', () => {
		const result = renderDocsPageResult(SAMPLE, 'Does not exist')
		expect(result).toContain('No section matching "Does not exist" was found')
		expect(result).toContain('- Jobs (~')
	})
})
