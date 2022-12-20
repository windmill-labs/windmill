<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import FlowModuleScript from '$lib/components/flows/content/FlowModuleScript.svelte'
	import { getScriptByPath } from '$lib/utils'
	import { faCodeBranch } from '@fortawesome/free-solid-svg-icons'
	import { r } from 'svelte-highlight/languages'
	import type { ResultAppInput } from '../../inputType'
	import { clearResultAppInput } from '../../utils'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'

	export let componentInput: ResultAppInput

	async function fork(path: string) {
		const { content, language, schema } = await getScriptByPath(path)
		componentInput.runnable = {
			type: 'runnableByName',
			name: path,
			inlineScript: {
				content,
				language,
				schema,
				path
			}
		}
	}
</script>

{#if componentInput?.runnable?.type === 'runnableByName' && componentInput?.runnable?.name !== undefined}
	{#if componentInput.runnable.inlineScript}
		<InlineScriptEditor
			bind:inlineScript={componentInput.runnable.inlineScript}
			bind:name={componentInput.runnable.name}
			on:delete={() => {
				componentInput = clearResultAppInput(componentInput)
			}}
		/>
	{:else}
		<EmptyInlineScript
			name={componentInput.runnable.name}
			on:new={(e) => {
				if (componentInput?.runnable?.type === 'runnableByName') {
					componentInput.runnable.inlineScript = e.detail
				}
			}}
		/>
	{/if}
{:else if componentInput?.runnable?.type === 'runnableByPath' && componentInput?.runnable?.path}
	<div class="p-2 h-full flex flex-col gap-2 ">
		<div>
			<Button
				size="xs"
				startIcon={{ icon: faCodeBranch }}
				on:click={() => {
					if (componentInput.runnable?.type === 'runnableByPath') {
						fork(componentInput.runnable.path)
					}
				}}
			>
				Fork
			</Button>
		</div>
		<div class="border w-full">
			<FlowModuleScript path={componentInput.runnable.path} />
		</div>
	</div>
{/if}
