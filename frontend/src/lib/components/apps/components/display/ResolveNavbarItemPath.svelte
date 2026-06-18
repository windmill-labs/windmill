<script lang="ts">
	import { untrack } from 'svelte'
	import { type NavbarItem } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { initConfig } from '../../editor/appUtils'

	interface Props {
		navbarItem: NavbarItem
		id: string
		index: number
		resolvedPath?: string | undefined
	}

	let { navbarItem, id, index, resolvedPath = $bindable(undefined) }: Props = $props()

	let resolvedConfig = $state(initConfig({ path: untrack(() => navbarItem).path }, { path: untrack(() => navbarItem).path }))

	$effect.pre(() => {
		resolvedPath = (
			resolvedConfig?.path?.selected === 'href'
				? resolvedConfig?.path?.configuration?.href?.href
				: resolvedConfig?.path?.configuration?.app?.path +
					(resolvedConfig?.path?.configuration?.app?.queryParamsOrHash ?? '')
		) as string | undefined
	})
</script>

<ResolveConfig
	{id}
	key={'path'}
	extraKey={String(index)}
	bind:resolvedConfig={resolvedConfig.path}
	configuration={navbarItem.path}
/>
