<script module lang="ts">
	export function validateToolName(name: string) {
		return /^[a-zA-Z0-9_]+$/.test(name)
	}
</script>

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
