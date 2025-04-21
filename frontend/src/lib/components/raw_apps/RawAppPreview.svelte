<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import type { HiddenRunnable } from '../apps/types'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import { htmlContent } from './utils'

	export let workspace: string
	export let user: UserExt | undefined
	export let version: number
	export let path: string
	export let runnables: Record<string, HiddenRunnable>

	let iframe: HTMLIFrameElement
</script>

<RawAppBackgroundRunner {workspace} editor={false} {iframe} {runnables} {path} />

<iframe
	bind:this={iframe}
	title="raw-app"
	srcDoc={htmlContent(workspace, version, { ctx: user, workspace })}
	class="w-full h-full min-h-screen bg-white border-none"
></iframe>
