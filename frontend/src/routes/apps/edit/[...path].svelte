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
						inputs: {
							a: {
								type: 'static',
								visible: true
							},
							b: {
								type: 'static',
								visible: true
							}
						},

						width: 33,
						horizontalAlignement: 'center',
						verticalAlignement: 'center',
						title: 'My title',
						description: 'My description',
						configSchema: undefined
					},
					{
						type: 'displaycomponent',
						id: 'b',
						width: 33,
						horizontalAlignement: 'center',
						verticalAlignement: 'center',
						title: 'My title',
						description: 'My description',
						configSchema: undefined,
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
				id: 'a',
				title: 'Section 1',
				description: 'Section 1 description'
			}
		],
		title: 'Fake title',
		policy: {
			triggerables: {
				a: {
					path: 'u/faton/my_script_3',
					staticFields: {
						a: 'a',
						b: 'b'
					},
					type: 'script'
				}
			}
		}
	}
</script>

<AppEditor bind:app />
