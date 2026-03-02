import { isRunnableByName, type EvalAppInput, type Runnable } from '$lib/components/apps/inputType'
import type { AppComponent } from '../../component'
import type {
	StaticAppInput,
	ConnectedAppInput,
	RowAppInput,
	UserAppInput
} from '$lib/components/apps/inputType'

export function getDependencies(
	fields:
		| Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput | EvalAppInput>
		| undefined
): string[] {
	let dependencies: string[] = []

	if (!fields) return dependencies

	Object.values(fields).forEach((field) => {
		if (field.type === 'connected' && dependencies && field.connection) {
			dependencies.push(`${field.connection.componentId} - ${field.connection.path}`)
		}

		if (field.type === 'eval' && !dependencies.includes('Eval')) {
			dependencies.push(`Eval`)
		}
	})
	return dependencies
}

export function isFrontend(runnable: Runnable): boolean {
	return isRunnableByName(runnable) && runnable.inlineScript?.language === 'frontend'
}

export function isTriggerable(componentType: string): boolean {
	return ['buttoncomponent', 'formbuttoncomponent', 'formcomponent', 'steppercomponent', 'chatcomponent'].includes(
		componentType
	)
}

export function isTriggerOnAppLoad(appComponent: AppComponent): boolean {
	return Boolean(
		appComponent?.configuration?.triggerOnAppLoad != undefined &&
			appComponent.configuration.triggerOnAppLoad.type == 'static' &&
			appComponent.configuration.triggerOnAppLoad.value
	)
}

export function getAllTriggerEvents(
	appComponent: AppComponent,
	autoRefresh: boolean | undefined
): string[] {
	const events: string[] = []
	const triggerOnAppLoad = isTriggerOnAppLoad(appComponent)
	const isTriggerableComponent = isTriggerable(appComponent.type)

	if (isTriggerableComponent) {
		// Use 'send' for chat component, 'click' for others
		if (appComponent.type === 'chatcomponent') {
			events.push('send')
		} else {
			events.push('click')
		}

		if (triggerOnAppLoad) {
			events.push('start')
		}
	} else if (autoRefresh) {
		events.push('start')
		events.push('refresh')
	}

	return events
}
