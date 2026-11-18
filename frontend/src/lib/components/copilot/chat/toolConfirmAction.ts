import { Check, Play, Plus, Save, Trash2, Undo2 } from 'lucide-svelte'
import type { ToolConfirmAction } from './shared'

export const TOOL_CONFIRM_ACTIONS: Record<
	ToolConfirmAction,
	{ label: string; icon: any; destructive?: boolean }
> = {
	run: { label: 'Run', icon: Play },
	deploy: { label: 'Deploy', icon: Save },
	create: { label: 'Create', icon: Plus },
	apply: { label: 'Apply', icon: Check },
	delete: { label: 'Delete', icon: Trash2, destructive: true },
	discard: { label: 'Discard', icon: Trash2, destructive: true },
	rollback: { label: 'Roll back', icon: Undo2 },
	confirm: { label: 'Confirm', icon: Check }
}
