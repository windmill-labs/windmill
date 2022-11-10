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
								value: 'Test value A',
								visible: true
							},
							b: {
								type: 'static',
								value: 'Test value B',
								visible: true
							}
						},

						width: 33,
						horizontalAlignement: 'center',
						verticalAlignement: 'center',
						title: 'My title',
						description: 'My description',
						configSchema: undefined,
						componentInputs: {}
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
						componentInputs: {
							result: {
								id: undefined,
								name: undefined,
								type: 'output',
								defaultValue: undefined
							}
						},
						inputs: {}
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
					type: 'script'
				}
			}
		}
	}
</script>

<AppEditor bind:app />
