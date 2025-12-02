<script lang="ts">
	import { Building, GitFork } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		workspace: any
		isFork: boolean
		className: string
	}

	let { workspace, isFork, className }: Props = $props()
</script>

<div class="flex items-center justify-between min-w-0 w-full">
	<div class="flex items-center gap-2 min-w-0">
		{#if isFork}
			<GitFork size={16} class={className} />
		{:else}
			<Building size={16} class={className} />
		{/if}
		<div class="min-w-0 flex-1">
			<div
				class={twMerge(
					'truncate text-left',
					className
				)}
			>
				{workspace.name}{workspace.disabled ? ' (user disabled)' : ''}
			</div>
			<div
				class={twMerge(
					'font-mono text-2xs whitespace-nowrap truncate text-left',
					className
				)}
			>
				{workspace.id}
			</div>
			<!-- {#if isForked && parentName} -->
			<!-- 	<div class="text-primary text-2xs truncate text-left pl-2 min-h-[1rem]"> -->
			<!-- 		Fork of {parentName} -->
			<!-- 	</div> -->
			<!-- {/if} -->
		</div>
	</div>
	{#if workspace.color}
		<div
			class="w-5 h-5 mr-2 rounded border border-gray-300 dark:border-gray-600"
			style="background-color: {workspace.color}"
		></div>
	{/if}
</div>
