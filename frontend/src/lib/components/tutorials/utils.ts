import type { Flow, FlowModule } from '$lib/gen'

export function setInputBySelector(selector: string, value: string) {
	const input = document.querySelector(selector) as HTMLInputElement

	if (input) {
		input.value = value
		input.dispatchEvent(new Event('input', { bubbles: true }))
	}
}

export function clickButtonBySelector(selector: string) {
	const button = document.querySelector(selector) as HTMLButtonElement

	if (button) {
		button.click()
	}
}

export function triggerAddFlowStep(index: number) {
	const button = document.querySelector(`#flow-editor-add-step-${index}`) as HTMLButtonElement

	if (button) {
		button.parentElement?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
	}
}

export function selectFlowStepKind(index: number) {
	const button = document.querySelector(
		`#flow-editor-insert-module > div > button:nth-child(${index})`
	) as HTMLButtonElement

	if (button) {
		button?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
	}
}

export function selectOptionsBySelector(selector: string, value: string) {
	const select = document.querySelector(selector) as HTMLSelectElement

	if (select) {
		select.value = value
		select.dispatchEvent(new Event('change', { bubbles: true }))
	}
}

export function isFlowTainted(flow: Flow) {
	return flow.value.modules.length > 0 || Object.keys(flow?.schema?.properties).length > 0
}

export function updateFlowModuleById(
	flow: Flow,
	id: string,
	callback: (module: FlowModule) => void
) {
	const dfs = (modules: FlowModule[]) => {
		for (const module of modules) {
			if (module.id === id) {
				callback(module)
				return
			}

			if (module.value.type === 'forloopflow') {
				dfs(module.value.modules)
			} else if (module.value.type === 'branchone') {
				module.value.branches.forEach((branch) => dfs(branch.modules))
			} else if (module.value.type === 'branchall') {
				module.value.branches.forEach((branch) => dfs(branch.modules))
			}
		}
	}

	dfs(flow.value.modules)

	flow = flow
}
