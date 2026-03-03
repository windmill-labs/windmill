<script lang="ts">
	interface Props {
		data: {
			headId: string
			type: string
			level: 'group' | 'branch' | 'loop-body'
			label: string
			wrapperWidth: number
			wrapperHeight: number
		}
	}
	let { data }: Props = $props()

	// Group wrappers: solid bg, thicker border
	// Branch wrappers: lighter bg, thinner border
	const groupColors: Record<string, { bg: string; border: string }> = {
		branchall: { bg: 'rgba(59, 130, 246, 0.08)', border: 'rgba(59, 130, 246, 0.5)' },
		branchone: { bg: 'rgba(168, 85, 247, 0.08)', border: 'rgba(168, 85, 247, 0.5)' },
		forloop: { bg: 'rgba(34, 197, 94, 0.08)', border: 'rgba(34, 197, 94, 0.5)' },
		whileloop: { bg: 'rgba(234, 179, 8, 0.08)', border: 'rgba(234, 179, 8, 0.5)' }
	}
	const branchColors: Record<string, { bg: string; border: string }> = {
		branchall: { bg: 'rgba(59, 130, 246, 0.12)', border: 'rgba(59, 130, 246, 0.7)' },
		branchone: { bg: 'rgba(168, 85, 247, 0.12)', border: 'rgba(168, 85, 247, 0.7)' },
		forloop: { bg: 'rgba(34, 197, 94, 0.12)', border: 'rgba(34, 197, 94, 0.7)' },
		whileloop: { bg: 'rgba(234, 179, 8, 0.12)', border: 'rgba(234, 179, 8, 0.7)' }
	}

	const fallback = { bg: 'rgba(100,100,100,0.06)', border: 'rgba(100,100,100,0.4)' }
	const colors = data.level === 'group'
		? (groupColors[data.type] ?? fallback)
		: (branchColors[data.type] ?? fallback)
	const borderWidth = data.level === 'group' ? 2 : 1.5
	const borderStyle = data.level === 'group' ? 'dashed' : 'dotted'
</script>

<div
	style="
		width: {data.wrapperWidth}px;
		height: {data.wrapperHeight}px;
		background: {colors.bg};
		border: {borderWidth}px {borderStyle} {colors.border};
		border-radius: {data.level === 'group' ? 10 : 6}px;
		pointer-events: none;
		position: relative;
	"
>
	<span
		style="
			position: absolute;
			{data.level === 'group' ? 'top: 2px; left: 6px;' : 'bottom: 2px; right: 6px;'}
			font-size: 9px;
			color: {colors.border};
			font-family: monospace;
			opacity: 0.85;
		"
	>
		{data.label} {data.wrapperWidth}×{data.wrapperHeight}
	</span>
</div>
