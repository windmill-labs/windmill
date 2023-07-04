<script lang="ts">
	import { Drawer, DrawerContent, Button } from '../common'
	import { generateScript } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import { sendUserToast } from '$lib/toast'
	import type Editor from '../Editor.svelte'

	// props
	export let lang: SupportedLanguage
	export let editor: Editor | undefined
	export let drawer: Drawer

	// state
	let funcDesc: string = ''
	let genLoading: boolean = false

	async function onGenerate() {
		try {
			genLoading = true
			const generatedCode = await generateScript({
				language: lang,
				description: funcDesc
			})

			editor?.setCode(generatedCode)
			drawer.closeDrawer()
			funcDesc = ''
		} catch (err) {
			sendUserToast('Failed to generate code')
			console.error(err)
		} finally {
			genLoading = false
		}
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Generate code" on:close={drawer.closeDrawer}
		><div class="flex gap-2 flex-col"
			><input
				bind:value={funcDesc}
				type="text"
				placeholder="Function description (e.g. print 'Hello World')"
			/>
			<Button size="xs" on:click={() => onGenerate()} loading={genLoading}>Generate</Button></div
		>
	</DrawerContent>
</Drawer>
