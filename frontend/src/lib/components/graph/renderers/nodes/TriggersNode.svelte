<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'

	export let data: {
		path: string
		openSchedules: () => void
		triggerDetail: (e: { detail: 'webhook' | 'mail' | 'schedule' }) => void
		isEditor: boolean
		newFlow: boolean
		extra_perms: Record<string, any>
	}
</script>

<NodeWrapper wrapperClass="shadow-none">
	<TriggersWrapper
		path={data.path}
		on:openSchedules={() => data.openSchedules()}
		on:triggerDetail={(e) => {
			data.triggerDetail(e.detail)
		}}
		isEditor={data.isEditor}
		newFlow={data.newFlow}
	/>
</NodeWrapper>
