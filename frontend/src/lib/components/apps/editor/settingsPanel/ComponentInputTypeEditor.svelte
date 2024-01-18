<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput, InputConnection } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { Code, CurlyBraces, FunctionSquare, Pen, Plug, Plug2 } from 'lucide-svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Button } from '$lib/components/common'
	import type EvalV2InputEditor from './inputEditor/EvalV2InputEditor.svelte'

	export let componentInput: AppInput
	export let disableStatic: boolean = false
	export let evalV2editor: EvalV2InputEditor | undefined

	const { onchange, connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

	$: if (componentInput.fieldType == 'template' && componentInput.type == 'static') {
		//@ts-ignore
		componentInput.type = 'templatev2'
		componentInput['eval'] = componentInput.value
		componentInput['connections'] = [{ componentId: 'ctx', id: 'email' }]
	}

	function applyConnection(connection: InputConnection) {
		const expr = `${connection.componentId}.${connection.path}`
		//@ts-ignore
		componentInput = {
			...componentInput,
			type: 'evalv2',
			expr: expr,
			connections: [{ componentId: connection.componentId, id: connection.path.split('.')[0] }]
		}
		evalV2editor?.setCode(expr)
		$app = $app
	}

	let clientWidth: number
</script>

{#if componentInput.fieldType !== 'any'}
	<div class="w-full">
		<div class="mx-auto flex gap-2" bind:clientWidth>
			<ToggleButtonGroup
				on:selected={() => {
					onchange?.()
				}}
				noWFull
				bind:selected={componentInput.type}
			>
				{#if componentInput.fieldType === 'template'}
					{#if componentInput.type == 'template'}
						<ToggleButton
							tooltip={`Templated string (use \$\{<output>.x\} )`}
							value="template"
							disabled={disableStatic}
							icon={CurlyBraces}
							label="Template"
						/>
					{:else}
						<ToggleButton
							tooltip={`Templated string (use \$\{<output>.x\} )`}
							value="templatev2"
							disabled={disableStatic}
							icon={CurlyBraces}
							label="Template"
						/>
					{/if}
				{:else if componentInput.noStatic !== true}
					<ToggleButton
						label="Static"
						value="static"
						disabled={disableStatic}
						iconOnly={clientWidth < 250}
						icon={Pen}
					/>
				{/if}
				{#if componentInput.type == 'connected'}
					<ToggleButton
						value="connected"
						icon={Plug2}
						iconOnly={clientWidth < 250}
						label="Connect"
					/>
				{/if}
				<ToggleButton
					value="evalv2"
					icon={FunctionSquare}
					iconOnly={clientWidth < 250}
					label="Eval"
				/>

				<ToggleButton
					value="runnable"
					icon={Code}
					iconOnly={clientWidth < 250}
					label="Compute"
					id="data-source-compute"
				/>
			</ToggleButtonGroup>
			<div class="flex">
				<Button
					size="xs"
					variant="border"
					color="light"
					title="Connect"
					id={`plug`}
					on:click={() => {
						$connectingInput = {
							opened: true,
							input: undefined,
							hoveredComponent: undefined,
							onConnect: applyConnection
						}
					}}
				>
					<Plug size={12} />
				</Button>
			</div>
		</div>
	</div>
{/if}
