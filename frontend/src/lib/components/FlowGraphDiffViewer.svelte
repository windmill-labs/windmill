<script lang="ts">
	import type { FlowValue } from '$lib/gen'
	import YAML from 'yaml'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { Alert } from './common'

	interface Props {
		beforeYaml: string
		afterYaml: string
	}

	let { beforeYaml, afterYaml }: Props = $props()

	let beforeFlow = $state<FlowValue | undefined>(undefined)
	let afterFlow = $state<FlowValue | undefined>(undefined)
	let parseError = $state<string | undefined>(undefined)

	$inspect('beforeYaml', beforeYaml)
	$inspect('afterYaml', afterYaml)

	// Parse YAML into FlowValue objects
	$effect(() => {
		try {
			parseError = undefined
			beforeFlow = YAML.parse(beforeYaml).value as FlowValue
			afterFlow = YAML.parse(afterYaml).value as FlowValue
		} catch (error) {
			parseError = error instanceof Error ? error.message : 'Failed to parse YAML'
			beforeFlow = undefined
			afterFlow = undefined
		}
	})
</script>

{#if parseError}
	<Alert type="error" title="Parse Error">
		{parseError}
	</Alert>
{:else if beforeFlow && afterFlow}
	<div class="grid grid-cols-2 h-full">
		<!-- Before (Left) -->
		<div class="flex flex-col h-full border-r border-gray-200 dark:border-gray-700">
			<div class="flex-1 overflow-hidden">
				<FlowGraphV2
					modules={beforeFlow.modules}
					failureModule={beforeFlow.failure_module}
					preprocessorModule={beforeFlow.preprocessor_module}
					earlyStop={beforeFlow.skip_expr !== undefined}
					cache={beforeFlow.cache_ttl !== undefined}
					insertable={false}
					editMode={false}
					download={false}
					scroll={false}
					minHeight={400}
					triggerNode={false}
				/>
			</div>
		</div>

		<!-- After (Right) -->
		<div class="flex flex-col h-full">
			<div class="flex-1 overflow-hidden">
				<FlowGraphV2
					modules={afterFlow.modules}
					failureModule={afterFlow.failure_module}
					preprocessorModule={afterFlow.preprocessor_module}
					earlyStop={afterFlow.skip_expr !== undefined}
					cache={afterFlow.cache_ttl !== undefined}
					notSelectable={true}
					insertable={false}
					editMode={false}
					download={false}
					scroll={false}
					minHeight={400}
					triggerNode={false}
				/>
			</div>
		</div>
	</div>
{:else}
	<div class="flex items-center justify-center h-full">
		<p class="text-gray-500">Loading graphs...</p>
	</div>
{/if}
