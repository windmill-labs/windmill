<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import type { App } from '$lib/components/apps/types'

	let app: App = {
		sections: [
			{
				components: [
					{
						id: 'a',
						type: 'runformcomponent',
						runType: 'script',
						path: 'u/faton/my_script_3',
						inputs: {
							runInputs: {
								a: {
									type: 'static',
									value: 'first'
								},
								b: {
									type: 'static',
									value: 'second'
								}
							}
						},
						params: {
							hidden: ['b']
						},
						width: 50,
						horizontalAlignement: 'center',
						verticalAlignement: 'center'
					},
					{
						type: 'displaycomponent',
						id: 'b',
						width: 50,
						inputs: {
							result: {
								id: 'a',
								name: 'result',
								type: 'output'
							}
						}
					}
				],
				columns: 3,
				id: 'a'
			}
		],
		title: 'Fake title'
	}
</script>

<AppEditor bind:app />
