<script lang="ts">
	import { type NavbarItem } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { initConfig } from '../../editor/appUtils'

	export let navbarItem: NavbarItem
	export let id: string
	export let index: number

	export let resolvedPath: string | undefined = undefined

	let resolvedConfig = initConfig({ path: navbarItem.path }, { path: navbarItem.path })

	$: resolvedPath = (
		resolvedConfig?.path?.selected === 'href'
			? resolvedConfig?.path?.configuration?.href?.href
			: resolvedConfig?.path?.configuration?.app?.path +
			  (resolvedConfig?.path?.configuration?.app?.queryParamsOrHash ?? '')
	) as string | undefined
</script>

<ResolveConfig
	{id}
	key={'path'}
	extraKey={String(index)}
	bind:resolvedConfig={resolvedConfig.path}
	configuration={navbarItem.path}
/>
