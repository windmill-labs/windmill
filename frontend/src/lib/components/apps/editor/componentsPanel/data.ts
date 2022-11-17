import type { AppComponent } from '../../types'
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
		},
		{
			...defaultProps,
			id: 'runformcomponent',
			type: 'runformcomponent'
		}
	] as AppComponent[]
}

const plainComponents = {
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
			type: 'buttoncomponent'
		},
		{
			...defaultProps,
			id: 'imagecomponent',
			type: 'imagecomponent'
		},
		{
			...defaultProps,
			id: 'inputcomponent',
			type: 'inputcomponent'
		},
		{
			...defaultProps,
			id: 'selectcomponent',
			type: 'selectcomponent'
		},
		{
			...defaultProps,
			id: 'checkboxcomponent',
			type: 'checkboxcomponent'
		},
		{
			...defaultProps,
			id: 'radiocomponent',
			type: 'radiocomponent'
		}
	] as AppComponent[]
}

const chartComponents = {
	title: 'Charts',
	components: [
		{
			...defaultProps,
			id: 'piechartcomponent',
			type: 'piechartcomponent'
		},
		{
			...defaultProps,
			id: 'barchartcomponent',
			type: 'barchartcomponent'
		}
	] as AppComponent[]
}

const componentSets = [windmillComponents, plainComponents, chartComponents]

export { componentSets }
