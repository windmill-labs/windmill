<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { AppViewerContext, InlineScript } from '$lib/components/apps/types'
	import { Button } from '$lib/components/common'
	import { ArrowRight } from 'lucide-svelte'
	import { getContext } from 'svelte'

	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let inlineScript: InlineScript | undefined = undefined

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')
</script>

<TriggerBadgesList
	{id}
	inputDependencies={getDependencies(fields)}
	frontendDependencies={inlineScript?.refreshOn?.map((x) => `${x.id} - ${x.key}`) ?? []}
	onLoad={autoRefresh}
	on:delete={(e) => {
		const index = e.detail.index
		if (inlineScript) {
			inlineScript.refreshOn?.splice(index, 1)
		}
	}}
>
	{#if inlineScript?.language === 'frontend'}
		<Button
			variant="border"
			size="xs"
			color="light"
			btnClasses="!px-2 !py-1"
			on:click={() => {
				$connectingInput = {
					opened: true,
					input: undefined,
					hoveredComponent: undefined
				}
			}}
		>
			<div class="flex flex-row gap-1 items-center">
				Connect
				<ArrowRight size={14} />
			</div>
		</Button>
	{/if}
</TriggerBadgesList>
