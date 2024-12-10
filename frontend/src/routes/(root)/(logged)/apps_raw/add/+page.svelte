<script>
	import 'sandpack-lit/dist/presets/sandpack'
	import 'sandpack-lit/dist/themes/odyssey.css'
</script>

<main>
	<div class="container">
		<!-- <sandpack-preset
			options={{
				closableTabs: false,
				files: {
					/** files go here */
				},
				customSetup: {
					dependencies: {
						lit: '2.6.1'
					}
				}
			}}
		/> -->
		<sandpack-provider>
			<sandpack-layout>
				<sandpack-preview initMode="" />
			</sandpack-layout>
		</sandpack-provider>
		<!-- <sandpack-preview options={{ template: 'vite', initMode: 'lazy' }} /> -->
	</div>
</main>

<style>
	.container {
		width: 900px;
	}
</style>
