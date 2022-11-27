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
					defaultValue: {},
					fieldType: 'object'
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
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
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
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'blue',
					optionValuesKey: 'buttonColorOptions',
					defaultValue: 'blue'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'md',
					optionValuesKey: 'buttonSizeOptions',
					defaultValue: 'md'
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
					value: undefined,
					fieldType: 'textarea',
					defaultValue: 'Lorem ipsum'
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
					fieldType: 'boolean',
					visible: true,
					defaultValue: false
				},
				paginationEnabled: {
					type: 'static',
					value: false,
					fieldType: 'boolean',
					visible: true,
					defaultValue: false
				}
			},

			runnable: true,
			card: true,
			components: []
		}
	] as AppComponent[]
}

const componentSets = [windmillComponents, plainComponents, chartComponents, tableComponents]

export { componentSets }
