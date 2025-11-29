<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import type { Runnable } from './rawAppPolicy'
	import { htmlContent } from './utils'

	interface Props {
		workspace: string
		user: UserExt | undefined
		secret: string | undefined
		path: string
		runnables: Record<string, Runnable>
	}

	let { workspace, user, secret, path, runnables }: Props = $props()

	let iframe = $state() as HTMLIFrameElement | undefined
</script>

<RawAppBackgroundRunner {workspace} editor={false} {iframe} {runnables} {path} />

<iframe
	bind:this={iframe}
	title="raw-app"
	srcDoc={htmlContent(workspace, secret, { ctx: user, workspace })}
	class="w-full h-full min-h-screen bg-white border-none"
></iframe>
