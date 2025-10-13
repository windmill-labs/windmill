<script lang="ts">
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Section from '$lib/components/Section.svelte'
	import { HelpCircle } from 'lucide-svelte'
	import { Button, Drawer, Tab, Tabs } from '../../common'
	import DrawerContent from '../../common/drawer/DrawerContent.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	let drawer: Drawer

	export let text: string = 'Approval Help'
</script>

<Button
	size="xs"
	variant="border"
	color="light"
	on:click={() => {
		drawer.openDrawer()
	}}
	>{text} <HelpCircle size={12} />
</Button>

<Drawer bind:this={drawer}>
	<DrawerContent title="Suspend/Approval/Prompt help" on:close={drawer.closeDrawer}>
		<div class="flex flex-col gap-y-6 text-xs text-primary font-normal">
			<Section label="Form/Payload">
				To add a form, go to the <b>Form</b> tab, inside the Advanced {'->'} Suspend tab, and add a form.
				You can then get back the payloads using `resume` (single approver), or `resumes` (multiple approvers)
				in the next step. Forms are an EE feature only. The approver list itself is fetchable using `approvers`
			</Section>
			<Section label="Prompt">
				A prompt is simply an approval step that can be self-approved. To do this, include the
				resume url in the returned payload of the step. The UX will automatically adapt and show the
				prompt to the operator when running the flow. Additionally, adding the cancel url will also
				render a cancel button, providing the operator with an option to cancel the step. e.g:
				<Tabs selected="bun" class="pt-4">
					<Tab value="bun">TypeScript (Bun)</Tab>
					<Tab value="deno">TypeScript (Deno)</Tab>
					<Tab value="python">Python</Tab>

					{#snippet content()}
						<TabContent value="deno" class="p-2">
							<HighlightCode
								language={'deno'}
								code={`import * as wmill from "npm:windmill-client@^1.158.2"
    
export async function main() {
    const urls = await wmill.getResumeUrls("approver1")

    return {
        resume: urls['resume'],
        cancel: urls['cancel'], 
        default_args: {}, // optional, see below
        enums: {} // optional, see below
    }
}`}
							/>
						</TabContent>
						<TabContent value="bun" class="p-2">
							<HighlightCode
								language={'deno'}
								code={`import * as wmill from "windmill-client"
        
export async function main() {
    const urls = await wmill.getResumeUrls("approver1")

    return {
        resume: urls['resume'],
        cancel: urls['cancel'],
        default_args: {}, // optional, see below
        enums: {} // optional, see below
    }
}`}
							/>
						</TabContent>
						<TabContent value="python" class="p-2">
							<HighlightCode
								language={'python3'}
								code={`import wmill

def main():
    urls = wmill.get_resume_urls()
    return {
        "resume": urls["resume"],
        "cancel": urls["cancel"],
        "default_args": {}, # optional, see below
        "enums": {} # optional, see below
    }
                                    `}
							/>
						</TabContent>
					{/snippet}
				</Tabs>
			</Section>
			<Section label="Default args">
				As one of the return key of this step, return an object `default_args` that contains the
				default arguments of the form arguments. e.g:
				<HighlightCode
					language={'deno'}
					code={`//this assumes the Form tab has a string field named "foo" and a checkbox named "bar"

import * as wmill from "npm:windmill-client@^1.158.2"

export async function main() {
    // if no argument is passed, if user is logged in, it will use the user's username
    const urls = await wmill.getResumeUrls("approver1") 

    // send the resumeUrls to the recipient or see Prompt section above

    return {
        default_args: {
            foo: "foo",
            bar: true
        }
    }
}`}
				/>
			</Section>
			<Section label="Dynamics enums">
				As one of the return key of this step, return an object `enums` that contains the default
				options of the form arguments. e.g:
				<HighlightCode
					language={'deno'}
					code={`

//this assumes the Form tab has a string field named "foo"

import * as wmill from "npm:windmill-client@^1.158.2"

export async function main() {
    // if no argument is passed, if user is logged in, it will use the user's username
    const url = await wmill.getResumeUrls("approver1") 

    // send the resumeUrls to the recipient or see Prompt section above

    return {
        enums: {
            foo: ["choice1", "choice2"]
        },
    }
}`}
				/>
			</Section>
		</div>
	</DrawerContent>
</Drawer>
