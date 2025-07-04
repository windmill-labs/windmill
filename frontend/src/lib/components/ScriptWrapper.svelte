<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import type { ScriptBuilderProps } from './script_builder'

    let {
        script: oldScript,
        ...props
    }: ScriptBuilderProps = $props()

    let script = $state(oldScript)
</script>


<ScriptBuilder
    {script}
    {...props}
/>
