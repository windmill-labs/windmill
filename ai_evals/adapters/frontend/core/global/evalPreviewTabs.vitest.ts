import { expect, it, vi } from 'vitest'

// The panel pulls in the global tool module, which reaches the editor stack it never uses here.
vi.mock('monaco-editor', () => ({
	editor: {},
	languages: {},
	KeyCode: {},
	Uri: { parse: (value: string) => ({ toString: () => value }) },
	MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 }
}))
vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features', () => ({
	getTypeScriptWorker: async () => async () => ({}),
	typescriptVersion: 'test'
}))
vi.mock('@codingame/monaco-vscode-languages-service-override', () => ({ default: () => ({}) }))
vi.mock('$lib/components/vscode', () => ({}))

const { createEvalPreviewPanel } = await import('./evalPreviewTabs')

// Every open makes its own tab active, so a fixture's `active` flag only means anything if
// it survives the tabs seeded after it. Lose that and a case still runs — against a panel
// state its author never described.
it('keeps the tab a fixture marks active, not the last one seeded', () => {
	const panel = createEvalPreviewPanel({
		sessionId: 'eval-preview-tabs-unit-test',
		tabs: [
			{ page: { href: '/runs', label: 'Runs' }, active: true },
			{ artifact: { name: 'Onboarding plan' } }
		],
		artifactIds: new Map([['Onboarding plan', 'eval-artifact-0']])
	})
	try {
		expect(panel.activePreview()?.location).toBe('/runs')
	} finally {
		panel.dispose()
	}
})
