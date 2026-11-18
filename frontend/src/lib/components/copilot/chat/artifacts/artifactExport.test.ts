import { describe, expect, it, vi } from 'vitest'
import { artifactHtmlDocument, renderArtifactHtml } from './artifactExport'

// The export never mounts the fenced-code renderer: keep its component tree out of the test.
vi.mock('$lib/components/MarkdownCodeBlock.svelte', () => ({ default: {} }))

describe('artifact HTML export', () => {
	it('keeps the content but drops scripts, handlers and javascript: links', () => {
		const html = renderArtifactHtml(
			[
				'<script>alert(1)</script>',
				'<img src="x.png" onerror="alert(2)">',
				'[link](javascript:alert(3))',
				'<a href="https://windmill.dev" onclick="alert(4)">site</a>'
			].join('\n\n')
		)
		expect(html).not.toMatch(/<script|onerror|onclick|javascript:/i)
		expect(html).toContain('<img src="x.png">')
		expect(html).toContain('<a href="https://windmill.dev">site</a>')
	})

	it('marks table cells after the sanitize stage, which would strip the classes', () => {
		const html = renderArtifactHtml(`| a | ${'long '.repeat(6)}|\n| - | - |\n| b | c |`)
		expect(html).toContain('<th class="wm-cell-short">a</th>')
		expect(html).toContain('<th class="wm-cell-long">')
		expect(html).toContain('<td class="wm-cell-short">c</td>')
	})

	it('leaves the section numbers to a document whose headings carry their own', () => {
		for (const heading of ['19th century', '2024', '1.5 release', '1,000 users']) {
			expect(renderArtifactHtml(`## Gaul\n\n## ${heading}`)).not.toContain('wm-unnumbered')
		}
		for (const heading of ['1. Goals', '1 Goals', '1️⃣ Goals']) {
			expect(renderArtifactHtml(`## Summary\n\n## ${heading}`)).toContain(
				'<h2 class="wm-unnumbered">Summary</h2>'
			)
		}
	})

	it('escapes the name it writes into the page title', () => {
		const page = artifactHtmlDocument('</title><script>alert(1)</script>', '# Hi')
		expect(page).toContain('<title>&lt;/title&gt;&lt;script&gt;alert(1)&lt;/script&gt;</title>')
		expect(page).not.toMatch(/<script/i)
	})
})
