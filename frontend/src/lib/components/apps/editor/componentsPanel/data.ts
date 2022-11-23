import type { AppComponent, ComponentSet } from '../../types'
import { defaultAlignement, defaultProps } from './componentDefaultProps'

const windmillComponents = {
	title: 'Windmill Components',
	components: [
		{
			...defaultProps,
			id: 'displaycomponent',
			type: 'displaycomponent',
			componentInputs: {
				result: {
					id: undefined,
					name: undefined,
					type: 'output',
					defaultValue: undefined
				}
			}
		}
	] as AppComponent[]
}

const plainComponents: ComponentSet = {
	title: 'Plain Components',
	components: [
		{
			...defaultProps,
			...defaultAlignement,
			id: 'textcomponent',
			type: 'textcomponent',
			componentInputs: {
				content: {
					type: 'static',
					visible: true,
					value: 'Lorem ipsum',
					fieldType: 'textarea'
				}
			}
		},
		{
			...defaultProps,
			id: 'buttoncomponent',
			type: 'buttoncomponent',
			componentInputs: {
				label: {
					type: 'static',
					visible: true,
					value: 'Lorem ipsum',
					fieldType: 'textarea'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'blue',
					optionValuesKey: 'buttonColorOptions'
				}
			},
			runnable: true
		},
		{
			...defaultProps,
			...defaultAlignement,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent',
			componentInputs: {
				label: {
					type: 'static',
					visible: true,
					value: 'Lorem ipsum',
					fieldType: 'textarea'
				}
			}
		}
	]
}

const chartComponents = {
	title: 'Charts',
	components: [
		{
			...defaultProps,
			id: 'piechartcomponent',
			type: 'piechartcomponent',
			runnable: true,
			card: true
		},
		{
			...defaultProps,
			id: 'barchartcomponent',
			type: 'barchartcomponent',
			runnable: true,
			card: true
		}
	] as AppComponent[]
}

const tableComponents = {
	title: 'Table',
	components: [
		{
			...defaultProps,
			id: 'tablecomponent',
			type: 'tablecomponent',
			componentInputs: {
				searchEnabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				},
				paginationEnabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean'
				}
			},
			runnable: true,
			card: true
		}
	] as AppComponent[]
}

const componentSets = [windmillComponents, plainComponents, chartComponents, tableComponents]

export { componentSets }
