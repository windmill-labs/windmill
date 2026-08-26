import type { FlowModule } from '$lib/gen'

/**
 * What to call a step that has no summary. The id checks sit between the composite types and
 * the leaf script types on purpose: a failure module holds a rawscript, and reading it as
 * "Inline python3 script" loses the only thing that distinguishes it.
 */
export function stepLabel(step: FlowModule): string {
	if (step.summary) return step.summary

	const value = step.value as Record<string, any> | undefined
	const suffixes = (...flags: [boolean | undefined, string][]) =>
		flags
			.filter(([on]) => on)
			.map(([, label]) => ` (${label})`)
			.join('')

	switch (value?.type) {
		case 'identity':
			return 'Identity'
		case 'forloopflow':
			return (
				'For loop' +
				suffixes(
					[value.parallel, 'parallel'],
					[value.skip_failures, 'skip failures'],
					[value.squash, 'squash']
				)
			)
		case 'branchall':
			return 'Run all branches' + suffixes([value.parallel, 'parallel'])
		case 'branchone':
			return 'Run one branch'
		case 'flow':
			return 'Inner flow'
		case 'whileloopflow':
			return (
				'While loop' + suffixes([value.skip_failures, 'skip failures'], [value.squash, 'squash'])
			)
	}

	if (step.id === 'failure') return 'Error handler'
	if (step.id === 'preprocessor') return 'Preprocessor'

	switch (value?.type) {
		case 'rawscript':
			return `Inline ${value.language} script`
		case 'script':
			return 'Workspace script'
		case 'aiagent':
			return 'AI Agent'
	}

	return ''
}
