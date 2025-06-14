<script lang="ts">
	import { getContext } from 'svelte'
	import type { ResultAppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { clearResultAppInput } from '../../utils'
	import type { AppComponent } from '../component'
	import ComponentScriptSettings, { type ActionType } from './script/ComponentScriptSettings.svelte'
	import { ExternalLink, X } from 'lucide-svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	interface Props {
		appInput: ResultAppInput
		appComponent: AppComponent
	}

	let { appInput = $bindable(), appComponent = $bindable() }: Props = $props()

	$effect.pre(() => {
		if (appInput.autoRefresh === undefined) {
			appInput.autoRefresh = true
		}
	})

	function detach() {
		if (appInput.runnable?.type === 'runnableByName' && appInput.runnable.inlineScript) {
			app.unusedInlineScripts.push({
				name: appInput.runnable.name,
				inlineScript: appInput.runnable.inlineScript
			}) // $app = $app
			appInput = clearResultAppInput(appInput)
		}
	}

	function clear() {
		appInput = clearResultAppInput(appInput)
	}

	$effect.pre(() => {
		if (appInput.recomputeOnInputChanged === undefined) {
			appInput.recomputeOnInputChanged = true
		}
		//TODO: remove after migration is done
		if (appInput.doNotRecomputeOnInputChanged != undefined) {
			appInput.recomputeOnInputChanged = !appInput.doNotRecomputeOnInputChanged
			appInput.doNotRecomputeOnInputChanged = undefined
		}
	})

	let hasScript = $derived(
		appInput?.runnable?.type === 'runnableByPath' ||
			(appInput?.runnable?.type === 'runnableByName' &&
				appInput.runnable?.inlineScript !== undefined)
	)

	function getActions(_hasScript: boolean): ActionType[] {
		return [
			...(appInput.runnable?.type === 'runnableByName' && appInput.runnable.inlineScript
				? ([
						{
							label: 'Detach',
							icon: ExternalLink,
							color: 'light',
							callback: detach
						}
					] as const)
				: []),
			{
				label: 'Clear',
				icon: X,
				color: 'red',
				callback: clear
			}
		]
	}

	let actions = $derived(getActions(hasScript))
</script>

<ComponentScriptSettings bind:appInput bind:appComponent {hasScript} {actions} />
