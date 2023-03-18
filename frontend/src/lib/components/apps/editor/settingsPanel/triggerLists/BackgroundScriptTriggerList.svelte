<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { AppViewerContext, InlineScript } from '$lib/components/apps/types'
	import { Button } from '$lib/components/common'
	import { Plus } from 'lucide-svelte'
	import { getContext } from 'svelte'

	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let inlineScript: InlineScript

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	let addingDependency: boolean = false

	function applyConnection() {
		if (!$connectingInput.opened && $connectingInput.input !== undefined && addingDependency) {
			if ($connectingInput.input.connection) {
				const x = {
					id: $connectingInput.input.connection.componentId,
					key: $connectingInput.input.connection.path
				}

				if (!inlineScript) {
					return
				}

				if (inlineScript.refreshOn?.find((y) => y.id === x.id && y.key === x.key)) {
					return
				}

				if (!inlineScript.refreshOn) {
					inlineScript.refreshOn = [x]
				} else {
					inlineScript.refreshOn.push(x)
				}

				inlineScript = JSON.parse(JSON.stringify(inlineScript))

				addingDependency = false
			}

			$connectingInput = {
				opened: false,
				input: undefined,
				hoveredComponent: undefined
			}
		}
	}

	$: $connectingInput && applyConnection()
</script>

<TriggerBadgesList
	{id}
	inputDependencies={getDependencies(fields)}
	frontendDependencies={inlineScript?.language === 'frontend'
		? inlineScript?.refreshOn?.map((x) => `${x.id} - ${x.key}`) ?? []
		: undefined}
	onLoad={autoRefresh}
	on:delete={(e) => {
		const index = e.detail.index
		if (inlineScript) {
			inlineScript.refreshOn?.splice(index, 1)
		}

		inlineScript = JSON.parse(JSON.stringify(inlineScript))
	}}
>
	{#if inlineScript?.language === 'frontend'}
		<Button
			variant="border"
			size="xs"
			color="light"
			btnClasses="!px-1 !py-0.5"
			on:click={() => {
				addingDependency = true
				$connectingInput = {
					opened: true,
					input: undefined,
					hoveredComponent: undefined
				}
			}}
		>
			<div class="flex flex-row gap-1 items-center">
				Add dependency
				<Plus size={14} />
			</div>
		</Button>
	{/if}
</TriggerBadgesList>
