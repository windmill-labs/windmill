<script lang="ts">
	import type { ComponentCustomCSS, RichConfigurations } from '../../../types'
	import type { AppInput, RunnableByName } from '../../../inputType'
	import { Preview } from '$lib/gen'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'tablecomponent'> | undefined = undefined
	export let render: boolean

	const runnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: ``,
			language: Preview.language.POSTGRESQL
		}
	}
</script>

<span>db explorer</span>
