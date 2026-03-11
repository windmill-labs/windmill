<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import GroupModuleIcons from '../../GroupModuleIcons.svelte'

	interface Props {
		data: CollapsedGroupN['data']
	}

	let { data }: Props = $props()
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<div
			class="w-full module cursor-pointer max-w-full"
			style="width: 275px;"
		>
			{#if data.modules && data.modules.length > 0}
				<div class="flex items-center justify-center w-full gap-1.5 px-2 h-[34px]">
					<GroupModuleIcons modules={data.modules} />
				</div>
			{/if}
		</div>
	{/snippet}
</NodeWrapper>
