<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import AssistantMessage from '$lib/components/copilot/chat/AssistantMessage.svelte'
	import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
	import DraggableTabs, { type TabItem } from '$lib/components/common/tabs/DraggableTabs.svelte'
	import { Globe } from 'lucide-svelte'

	let tab = $state('button')

	// Enough tabs to overflow a narrow strip so the shared ScrollableX hover
	// scrollbar is exercised: drag to reorder, hover to reveal the 4px thumb.
	let draggableTabs = $state<TabItem[]>(
		Array.from({ length: 14 }, (_, i) => ({ id: `t${i}`, label: `Preview tab ${i + 1}` }))
	)
	let activeDraggableTab = $state('t0')

	const sampleMarkdown = `# Heading 1

## Heading 2

Body text with **bold**, *italic*, a [link](https://windmill.dev), and \`inline code\` that must stay readable in both themes.

> A block quote should be legible too.

- First bullet
- Second bullet with \`code\`

1. Ordered one
2. Ordered two

\`\`\`ts
const block = 'code block'
console.log(block)
\`\`\`

| Column A | Column B |
| -------- | -------- |
| one      | two      |
| three    | four     |

---
`
	let dropdownItems = [
		{
			label: 'Lorem ipsum',
			onClick: () => {}
		}
	]

	// Mirrors how the AI chat renders assistant answers: markdown flows through
	// AssistantMessage, and fenced code blocks go through CodeDisplay →
	// HighlightCode. Exercise several languages + prose so the code-block styling
	// can be tuned against the real render path, not an approximation.
	const chatSampleContent = `Here's how you'd wire up the trigger. First, some prose with \`inline code\`, a [link](https://windmill.dev), and **bold** text so we can see how code sits next to surrounding content.

\`\`\`python
def main(name: str = "world"):
    # a short python snippet
    greeting = f"hello, {name}!"
    print(greeting)
    return {"greeting": greeting}
\`\`\`

A one-liner in the middle of a sentence like \`SELECT * FROM users\` should stay inline. Now a TypeScript block:

\`\`\`ts
export async function main(count: number) {
  const rows = await Promise.all(
    Array.from({ length: count }, (_, i) => fetchRow(i))
  )
  return rows.filter((r) => r.active).map((r) => r.id)
}
\`\`\`

A block with a very long line to check horizontal overflow behaviour:

\`\`\`bash
curl -sSL "https://app.windmill.dev/api/w/demo/jobs/run_wait_result/p/u/admin/very_long_script_path?token=abcdef0123456789&include_header=Authorization" | jq '.result.value'
\`\`\`

Some SQL:

\`\`\`sql
select id, name, created_at
from users
where active = true
order by created_at desc
limit 10;
\`\`\`

And a bulleted list where an item carries \`code\`:

- First item with \`some_var\`
- Second item
- Third item

\`\`\`rust
fn main() {
    let items = vec![1, 2, 3];
    let sum: i32 = items.iter().sum();
    println!("sum = {sum}");
}
\`\`\`

That's the full round-trip.`

	const chatMessage: DisplayMessage = {
		role: 'assistant',
		content: chatSampleContent
	}
</script>

<DarkModeToggle forcedDarkMode={false} />

<Tabs bind:selected={tab}>
	<Tab value="button" label="Buttons" />
	<Tab value="markdown" label="Markdown" />
	<Tab value="chat" label="AI Chat" />
	<Tab value="scrollbar" label="Scrollbar" />

	{#snippet content()}
		<TabContent value="button" class="p-4 flex gap-4 flex-col ">
			<div class="font-bold text-md">Contained buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button>Lorem</Button>
				<Button disabled>Lorem</Button>

				<Button variant="accent" loading>Lorem</Button>
				<Button
					color="gray"
					startIcon={{
						icon: Globe
					}}
				>
					Lorem
				</Button>
				<Button color="green">Lorem</Button>
				<Button color="light">Lorem</Button>
				<Button color="none">Lorem</Button>
				<Button color="red">Lorem</Button>
			</div>
			<div class="font-bold text-md">Border buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
				<Button variant="default">Lorem</Button>
			</div>
			<div class="font-bold text-md">Dropdown buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button {dropdownItems}>Lorem</Button>
				<Button {dropdownItems} variant="accent">Lorem</Button>
				<Button {dropdownItems} color="gray">Lorem</Button>
				<Button {dropdownItems} color="green">Lorem</Button>
				<Button {dropdownItems} color="light">Lorem</Button>
				<Button {dropdownItems} color="none">Lorem</Button>
				<Button {dropdownItems} color="red">Lorem</Button>
			</div>
			<div class="font-bold text-md">Dropdown buttons</div>
			<div class="grid grid-cols-2 gap-2 md:grid-cols-4 lg:grid-cols-6">
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
				<Button variant="default" {dropdownItems}>Lorem</Button>
			</div>
		</TabContent>
		<TabContent value="markdown" class="p-4">
			<GfmMarkdown md={sampleMarkdown} />
		</TabContent>
		<TabContent value="chat" class="p-4">
			<div class="text-xs text-tertiary mb-3">
				Rendered through the real chat path (<code>AssistantMessage</code> →
				<code>CodeDisplay</code> → <code>HighlightCode</code>), constrained to the chat panel width.
			</div>
			<div class="border border-border-light rounded-lg p-3 bg-surface" style="max-width: 420px;">
				<AssistantMessage message={chatMessage} />
			</div>
		</TabContent>
		<TabContent value="scrollbar" class="p-4">
			<div class="text-xs text-tertiary mb-3">
				DraggableTabs (uses the shared <code>ScrollableX</code>, 4px bar): hover to reveal the
				thumb, drag to reorder.
			</div>
			<div class="border border-border-light rounded-md" style="max-width: 420px;">
				<DraggableTabs
					tabs={draggableTabs}
					activeId={activeDraggableTab}
					onSelect={(id) => (activeDraggableTab = id)}
					onReorder={(next) => (draggableTabs = next)}
				/>
			</div>
		</TabContent>
	{/snippet}
</Tabs>
