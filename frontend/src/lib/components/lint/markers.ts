import { editor as meditor, MarkerSeverity, Uri } from 'monaco-editor'
import type { ScriptLintResult } from '../copilot/chat/shared'

// The single way lint results are read off a model, shared by the editor and by headless
// linting so the two can never diverge in what they count as an error or a warning.
export function readModelMarkers(uri: Uri): ScriptLintResult {
	const markers = meditor.getModelMarkers({ resource: uri })
	const errors = markers.filter((m) => m.severity === MarkerSeverity.Error)
	const warnings = markers.filter((m) => m.severity === MarkerSeverity.Warning)
	return { errorCount: errors.length, warningCount: warnings.length, errors, warnings }
}
