<script lang="ts">
	import { getContext, createEventDispatcher } from 'svelte'
	import type { AppInput, InputConnection } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { Code, CurlyBraces, FunctionSquare, Pen, Plug2 } from 'lucide-svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ConnectionButton from '$lib/components/common/button/ConnectionButton.svelte'
	import type EvalV2InputEditor from './inputEditor/EvalV2InputEditor.svelte'

	interface Props {
		componentInput: AppInput
		disableStatic?: boolean
		evalV2editor: EvalV2InputEditor | undefined
	}

	let { componentInput = $bindable(), disableStatic = false, evalV2editor }: Props = $props()

	const { onchange, connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

	const dispatch = createEventDispatcher()

	$effect(() => {
		if (componentInput.fieldType == 'template' && componentInput.type == 'static') {
			//@ts-ignore
			componentInput.type = 'templatev2'
			componentInput['eval'] = componentInput.value
			componentInput['connections'] = [{ componentId: 'ctx', id: 'email' }]
		}
	})

	function applyConnection(connection: InputConnection) {
		console.log(connection)
		const expr = `${connection.componentId}.${connection.path}`
		//@ts-ignore
		componentInput = {
			...componentInput,
			type: 'evalv2',
			expr: expr,
			connections: [
				{
					componentId: connection.componentId,
					id: connection?.path?.split('.')?.[0]?.split('[')?.[0]
				}
			]
		}
		evalV2editor?.setCode(expr)
		app.val = app.val
	}

	let clientWidth: number = $state(0)
	const iconOnlyThreshold = 300
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full">
		<div class="flex gap-2 justify-end" bind:clientWidth>
			<ToggleButtonGroup
				on:selected={() => {
					onchange?.()
				}}
				noWFull
				bind:selected={componentInput.type}
			>
				{#snippet children({ item })}
					{#if componentInput.fieldType === 'template'}
						{#if componentInput.type == 'template'}
							<ToggleButton
								tooltip={`Templated string (use \$\{<output>.x\} )`}
								value="template"
								disabled={disableStatic}
								icon={CurlyBraces}
								label="Template"
								{item}
							/>
						{:else}
							<ToggleButton
								tooltip={`Templated string (use \$\{<output>.x\} )`}
								value="templatev2"
								disabled={disableStatic}
								icon={CurlyBraces}
								label="Template"
								{item}
							/>
						{/if}
					{:else if componentInput.noStatic !== true}
						<ToggleButton
							label="Static"
							value="static"
							disabled={disableStatic}
							iconOnly={clientWidth < iconOnlyThreshold}
							icon={Pen}
							{item}
						/>
					{/if}
					{#if componentInput.type == 'connected'}
						<ToggleButton
							value="connected"
							icon={Plug2}
							iconOnly={clientWidth < iconOnlyThreshold}
							label="Connect"
							{item}
						/>
					{/if}
					<ToggleButton
						value="evalv2"
						icon={FunctionSquare}
						iconOnly={clientWidth < iconOnlyThreshold}
						label="Eval"
						{item}
					/>

					<ToggleButton
						value="runnable"
						icon={Code}
						iconOnly={clientWidth < iconOnlyThreshold}
						label="Compute"
						{item}
						id="data-source-compute"
					/>
				{/snippet}
			</ToggleButtonGroup>

			<div class="flex">
				<ConnectionButton
					closeConnection={() => {
						$connectingInput = {
							opened: false,
							hoveredComponent: undefined,
							input: undefined,
							onConnect: () => {}
						}
						dispatch('select', true)
					}}
					openConnection={() => {
						$connectingInput = {
							opened: true,
							input: undefined,
							hoveredComponent: undefined,
							onConnect: applyConnection
						}
					}}
					isOpen={!!$connectingInput.opened}
				/>
			</div>
		</div>
	</div>
{/if}
