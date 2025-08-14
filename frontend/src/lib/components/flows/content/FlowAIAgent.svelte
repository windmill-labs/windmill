<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import FlowModuleComponent from './FlowModuleComponent.svelte'

	interface Props {
		flowModule: FlowModule
		noEditor: boolean
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		scriptKind?: 'script' | 'trigger' | 'approval'
		scriptTemplate?: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell'
		enableAi: boolean
		savedModule: FlowModule | undefined
		forceTestTab?: boolean
		highlightArg?: string
	}

	let {
		flowModule = $bindable(),
		noEditor,
		previousModule,
		parentModule,
		scriptKind,
		scriptTemplate,
		enableAi,
		savedModule,
		forceTestTab,
		highlightArg
	}: Props = $props()

	let fakeFlowModule: FlowModule = $derived({
		id: flowModule.id,
		value: {
			type: 'rawscript',
			input_transforms:
				flowModule.value.type === 'aiagent' ? flowModule.value.input_transforms : {},
			content:
				'//native\nexport async function main(system_prompt: string, user_message: string, resource: RT.Openai, model: string = "gpt-4o-mini") {}\n',
			language: 'nativets'
		}
	})
</script>

<FlowModuleComponent
	bind:flowModule
	{noEditor}
	{previousModule}
	{parentModule}
	{scriptKind}
	{scriptTemplate}
	{enableAi}
	{savedModule}
	{forceTestTab}
	{highlightArg}
/>
