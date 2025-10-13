<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import AiChatLayout from './copilot/chat/AiChatLayout.svelte'
	import type { ScriptBuilderProps } from './script_builder'

	let { script: oldScript, disableAi, ...props }: ScriptBuilderProps = $props()

	let script = $state(oldScript)
</script>

<AiChatLayout noPadding {disableAi}>
	<ScriptBuilder bind:script {disableAi} {...props} />
</AiChatLayout>
