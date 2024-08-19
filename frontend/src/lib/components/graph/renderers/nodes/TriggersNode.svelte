<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'

	export let data: {
		path: string
	}
</script>

<NodeWrapper wrapperClass="shadow-none">
	<TriggersWrapper path={data.path} />
</NodeWrapper>
