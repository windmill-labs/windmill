export type VariableSecretDraft = {
	value: string
	is_secret: boolean
}

export function shouldBlockUnsecretWithoutLoad(args: {
	edit: boolean
	deployedSecret: boolean
	variable: VariableSecretDraft
}): boolean {
	const { edit, deployedSecret, variable } = args
	const hasStagedValue = variable.value !== ''
	return edit && deployedSecret && variable.is_secret && !hasStagedValue
}

export function applyVariableSecretToggle(args: {
	isSecret: boolean
	deployedSecret: boolean
	draft: VariableSecretDraft
	deployedBaseline: VariableSecretDraft
}): { value: string; baselineValue: string } | null {
	const { isSecret, deployedSecret, draft, deployedBaseline } = args
	if (!deployedBaseline.is_secret) return null

	if (!isSecret) {
		const revealedWithoutAudit =
			deployedSecret && deployedBaseline.value === '' && draft.value !== ''
		if (!revealedWithoutAudit) return null
		return { value: '', baselineValue: deployedBaseline.value }
	}

	const mustRemask = deployedBaseline.value === '' && draft.value !== ''
	if (!mustRemask && draft.value !== '' && draft.value !== deployedBaseline.value) {
		return null
	}
	return { value: '', baselineValue: '' }
}
