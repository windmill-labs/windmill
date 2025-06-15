<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import type { AppInput, InputConnectionEval } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import InputValue from './InputValue.svelte'
	import InitializeComponent from './InitializeComponent.svelte'
	import RefreshIndicator from './RefreshIndicator.svelte'

	interface Props {
		componentInput: AppInput
		id: string
		result: any
		render: boolean
		hasChildrens: boolean
		noInitialize: any
		children?: import('svelte').Snippet
	}

	let {
		componentInput,
		id,
		result = $bindable(),
		render,
		hasChildrens,
		noInitialize,
		children
	}: Props = $props()

	// Sync the result to the output
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = $derived(
		$worldStore?.outputsById?.[id] as {
			result: Output<any>
		}
	)

	function setOutput(v: any) {
		// console.log('setnr', id)
		outputs?.result?.set(v, true)
	}

	let loading: boolean = $state(false)
	let loadingStates: { [key: string]: boolean } = {}
	let subscriptions: Array<() => void> = []

	function updateGlobalLoading() {
		loading = Object.values(loadingStates).some((state) => state === true)
	}

	function builtSubscriptions(connections: InputConnectionEval[]) {
		// If we are rebuilding the subscriptions, we need to unsubscribe from the previous ones
		if (subscriptions.length > 0) {
			subscriptions.forEach((unsubscribe) => unsubscribe?.())
			subscriptions = []
			loadingStates = {}
			loading = false
		}

		connections.forEach((connection) => {
			const output = $worldStore.outputsById[connection.componentId]

			if (output?.loading?.subscribe) {
				const unsubscribe = output.loading.subscribe(
					{
						id: `loading-${connection.componentId}`,
						next: (isConnectionLoading: boolean) => {
							loadingStates[connection.componentId] = isConnectionLoading
							updateGlobalLoading()
						}
					},
					loadingStates[connection.componentId]
				)
				subscriptions.push(unsubscribe)
			}
		})
	}

	$effect.pre(() => {
		componentInput.type === 'evalv2' &&
			componentInput.connections &&
			untrack(() => builtSubscriptions(componentInput.connections))
	})

	$effect.pre(() => {
		result != undefined && outputs && untrack(() => setOutput(result))
	})
</script>

{#if !noInitialize}
	<InitializeComponent {id} />
{/if}

{#if componentInput.type !== 'runnable'}
	<InputValue key="nonrunnable" {id} input={componentInput} bind:value={result} />
{/if}

{#if render || hasChildrens}
	<div class={render ? 'h-full w-full' : 'invisible h-0 overflow-hidden'}>
		{@render children?.()}
		<div class="flex absolute top-1 right-1 z-50 app-component-refresh-btn">
			<RefreshIndicator {loading} />
		</div>
	</div>
{/if}
