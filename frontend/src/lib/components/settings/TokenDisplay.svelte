<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import Alert from '../common/alert/Alert.svelte'

	interface Props {
		token: string
		type?: 'token' | 'mcp'
		mcpUrl?: string
		title?: string
		onCopy?: () => void
	}

	let {
		token,
		type = 'token',
		mcpUrl,
		title,
		onCopy
	}: Props = $props()

	function handleCopyClick() {
		copyToClipboard(token)
		onCopy?.()
	}

	const displayTitle = $derived(
		title || (type === 'mcp' ? 'MCP URL Generated Successfully' : 'Token Created Successfully')
	)

	const colorScheme = {
		gradient: 'from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20',
		border: 'border-blue-200 dark:border-blue-700',
		iconBg: 'bg-blue-100 dark:bg-blue-800',
		iconColor: 'text-blue-600 dark:text-blue-300',
		titleColor: 'text-blue-800 dark:text-blue-200',
		labelColor: 'text-blue-700 dark:text-blue-300',
		infoBg: 'bg-blue-50 dark:bg-blue-900/30',
		infoBorder: 'border-blue-200 dark:border-blue-600',
		infoText: 'text-blue-700 dark:text-blue-300'
	}
</script>

<div class="border rounded-lg mb-6 p-4 bg-gradient-to-r {colorScheme.gradient} {colorScheme.border} shadow-sm">
	<div class="flex items-start gap-3">
		<div class="flex-shrink-0 w-8 h-8 {colorScheme.iconBg} rounded-full flex items-center justify-center mt-0.5">
			{#if type === 'mcp'}
				<svg class="w-4 h-4 {colorScheme.iconColor}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"></path>
				</svg>
			{:else}
				<svg class="w-4 h-4 {colorScheme.iconColor}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
				</svg>
			{/if}
		</div>
		<div class="flex-1 min-w-0">
			<h4 class="text-sm font-semibold {colorScheme.titleColor} mb-2">
				{displayTitle}
			</h4>
			
			{#if type === 'mcp' && mcpUrl}
				<div class="space-y-3">
					<div>
						<!-- svelte-ignore a11y_label_has_associated_control -->
						<label class="block text-xs font-medium {colorScheme.labelColor} mb-1">
							Your MCP Server URL:
						</label>
						<ClipboardPanel content={mcpUrl} />
					</div>
					<div>
						<!-- svelte-ignore a11y_label_has_associated_control -->
						<label class="block text-xs font-medium {colorScheme.labelColor} mb-1">
							Your Token:
						</label>
						<div class="bg-white dark:bg-gray-800 rounded-md p-3 border {colorScheme.border}">
							<div class="flex items-center justify-between gap-2">
								<code class="text-sm font-mono text-gray-800 dark:text-gray-200 break-all flex-1">
									{token}
								</code>
								<button 
									onclick={handleCopyClick} 
									class="flex-shrink-0 p-1.5 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
									title="Copy token"
								>
									<Clipboard size={16} />
								</button>
							</div>
						</div>
					</div>
					<Alert type="warning" title="Important" size="xs">
						Make sure to copy both the URL and token now. You won't be able to see them again!
					</Alert>
					<div class="{colorScheme.infoBg} rounded-md p-2 border {colorScheme.infoBorder}">
						<p class="text-xs {colorScheme.infoText}">
							<strong>Next steps:</strong> Use this URL in your MCP-compatible client (like Claude Desktop) to access your Windmill scripts and flows as tools.
						</p>
					</div>
				</div>
			{:else}
				<div class="bg-white dark:bg-gray-800 rounded-md p-3 border {colorScheme.border}">
					<div class="flex items-center justify-between gap-2">
						<code class="text-sm font-mono text-gray-800 dark:text-gray-200 break-all flex-1">
							{token}
						</code>
						<button 
							onclick={handleCopyClick} 
							class="flex-shrink-0 p-1.5 text-gray-500 hover:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
							title="Copy token"
						>
							<Clipboard size={16} />
						</button>
					</div>
				</div>
				<div class="mt-3">
					<Alert type="warning" title="Important" size="xs">
						Make sure to copy your personal access token now. You won't be able to see it again!
					</Alert>
				</div>
			{/if}
		</div>
	</div>
</div>