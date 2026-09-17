import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const here = dirname(fileURLToPath(import.meta.url))

// Components an AI session mounts for a workspace other than the navigation one: every trigger
// editor, and the pickers and forms they and the variable/resource editors are built from. One of
// them reading the navigation store writes to the parent from inside a fork's editor, and nothing
// else would catch it — so they read `useOperatingWorkspace()` instead.
const LEAVES = [
	'AIProviderPicker.svelte',
	'ArgInput.svelte',
	'ChannelSelector.svelte',
	'EditableSchemaForm.svelte',
	'ErrorOrRecoveryHandler.svelte',
	'ExploreAssetButton.svelte',
	'GitHubAppIntegration.svelte',
	'GitLabIntegration.svelte',
	'GitRepoResourcePicker.svelte',
	'HistoricInputs.svelte',
	'InputTransformPickers.svelte',
	'InputTransformSchemaForm.svelte',
	'LabelsInput.svelte',
	'LightweightResourcePicker.svelte',
	'PasswordArgInput.svelte',
	'Path.svelte',
	'PathNameAutocomplete.svelte',
	'ResourceEditor.svelte',
	'ResourceEditorDrawer.svelte',
	'ResourceForm.svelte',
	'ResourcePicker.svelte',
	'ResourceVersionHistory.svelte',
	'S3FilePicker.svelte',
	'S3FilePickerInner.svelte',
	'S3FilePreview.svelte',
	'SaveInputsButton.svelte',
	'SavedInputsPicker.svelte',
	'SchemaForm.svelte',
	'ScriptPicker.svelte',
	'TestConnection.svelte',
	'VariableEditor.svelte',
	'VariableForm.svelte'
]

function svelteFilesUnder(dir: string): string[] {
	return readdirSync(join(here, dir), { recursive: true, encoding: 'utf-8' })
		.filter((f) => f.endsWith('.svelte'))
		.map((f) => join(dir, f))
}

describe('components under a session editor', () => {
	it('read the operating workspace, never the navigation store', () => {
		const offenders = [...svelteFilesUnder('triggers'), ...LEAVES].filter((f) =>
			/\bworkspaceStore\b/.test(readFileSync(join(here, f), 'utf-8'))
		)
		expect(offenders).toEqual([])
	})
})
