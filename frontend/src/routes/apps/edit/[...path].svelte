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
						}
					},
					{
						type: 'displaycomponent',
						id: 'b',
						inputs: {
							result: {
								id: 'a',
								name: 'result',
								type: 'output'
							}
						}
					}
				],
				columns: 3
			}
		],
		title: 'Fake title'
	}
</script>

<AppEditor bind:app />
