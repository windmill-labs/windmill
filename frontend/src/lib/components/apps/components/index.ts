import { default as AppButton } from './buttons/AppButton.svelte'
export { default as AppButton } from './buttons/AppButton.svelte'
import { default as AppBarChart } from './dataDisplay/AppBarChart.svelte'
export { default as AppBarChart } from './dataDisplay/AppBarChart.svelte'
import { default as AppPieChart } from './dataDisplay/AppPieChart.svelte'
export { default as AppPieChart } from './dataDisplay/AppPieChart.svelte'
import { default as AppTable } from './dataDisplay/AppTable.svelte'
export { default as AppTable } from './dataDisplay/AppTable.svelte'
import { default as AppText } from './dataDisplay/AppText.svelte'
export { default as AppText } from './dataDisplay/AppText.svelte'
import { default as AppCheckbox } from './selectInputs/AppCheckbox.svelte'
export { default as AppCheckbox } from './selectInputs/AppCheckbox.svelte'

export { default as AlignWrapper } from './helpers/AlignWrapper.svelte'
export { default as DebouncedInput } from './helpers/DebouncedInput.svelte'
export { default as InputValue } from './helpers/InputValue.svelte'
export { default as RunnableComponent } from './helpers/RunnableComponent.svelte'

// Component groups
const textInputs = []
const numberInputs = []
const buttons = [
	AppButton
]
const selectInputs = [
	AppCheckbox
]
const dateTimeInputs = []
const dataDisplay = [
	AppBarChart,
	AppPieChart,
	AppTable,
	AppText
]

// Aggregated component groups for ease of import
const APP_COMPONENTS = [
	textInputs,
	numberInputs,
	buttons,
	selectInputs,
	dateTimeInputs,
	dataDisplay
]

export {
	APP_COMPONENTS,
	textInputs,
	numberInputs,
	buttons,
	selectInputs,
	dateTimeInputs,
	dataDisplay
}