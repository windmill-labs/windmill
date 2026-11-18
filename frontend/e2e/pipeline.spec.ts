import { test, expect, Page } from '@playwright/test'

// The pipeline surface an AI session builds into: session pipeline tools emit
// annotated scripts (`-- pipeline`, `-- on <asset>`, `-- materialize <asset>`),
// and the /pipeline/<folder> editor derives the lineage DAG from those
// annotations alone. This test seeds the scripts an AI-built DuckLake pipeline
// would produce and asserts the editor renders every derived node, asset, and
// edge (including the missing-schedule-trigger edge): a deterministic check of
// the graph the AI session relies on, without a live model in the loop.

const WORKSPACE = 'admins'

declare const process: any

function uniqueSuffix(project: string): string {
	// Per-project suffix so the three browser projects don't collide on the
	// shared dev instance when Playwright runs them in parallel.
	return `${process.env.TEST_UNIQUE_ID ?? 'local'}_${project}`
}

async function seedScript(page: Page, path: string, content: string, summary: string) {
	const res = await page.request.post(`/api/w/${WORKSPACE}/scripts/create`, {
		data: { path, summary, description: '', content, language: 'duckdb', schema: {} }
	})
	expect(res.ok(), `seed ${path}: ${res.status()} ${await res.text()}`).toBeTruthy()
}

test.describe('Pipeline editor', () => {
	// Track what the test seeds so afterAll can remove it (keeps the shared dev/CI
	// instance from accumulating a folder + scripts per run).
	let seeded: { folder: string; scripts: string[] } | undefined

	test.afterAll(async ({ request }) => {
		if (!seeded) return
		for (const path of seeded.scripts) {
			await request.post(`/api/w/${WORKSPACE}/scripts/delete/p/${path}`).catch(() => {})
		}
		await request.delete(`/api/w/${WORKSPACE}/folders/delete/${seeded.folder}`).catch(() => {})
	})

	test('derives the DAG from annotated pipeline scripts', async ({ page }, testInfo) => {
		const suffix = uniqueSuffix(testInfo.project.name)
		const folder = `pipeline_e2e_${suffix}`
		const ingest = `f/${folder}/orders_ingest`
		const daily = `f/${folder}/orders_daily`
		const ordersTbl = `main/orders_${suffix}`
		const dailyTbl = `main/orders_daily_${suffix}`
		seeded = { folder, scripts: [ingest, daily] }

		// Folder may already exist from a prior run; only fail on the seeds.
		await page.request.post(`/api/w/${WORKSPACE}/folders/create`, { data: { name: folder } })

		await seedScript(
			page,
			ingest,
			[
				'-- pipeline',
				'-- on schedule',
				`-- materialize ducklake://${ordersTbl}`,
				"SELECT * FROM read_csv('s3://raw/orders/*.csv')"
			].join('\n'),
			'Ingest orders'
		)
		await seedScript(
			page,
			daily,
			[
				'-- pipeline',
				`-- on ducklake://${ordersTbl}`,
				`-- materialize ducklake://${dailyTbl}`,
				`SELECT date_trunc('day', ts) AS day, count(*) AS n FROM ducklake.${ordersTbl.replace('/', '.')} GROUP BY 1`
			].join('\n'),
			'Daily rollup'
		)

		await page.goto(`/pipeline/${folder}`)

		await expect(page.getByRole('heading', { name: 'Pipeline', level: 1 })).toBeVisible()
		await expect(page.getByText('2 scripts', { exact: false })).toBeVisible()

		// A long path truncates in the node label, so match the leaf name; the full
		// paths are asserted on the edge labels below.
		await expect(page.getByText('orders_ingest').first()).toBeVisible()
		await expect(page.getByText('orders_daily').first()).toBeVisible()

		// Two edges resolve asynchronously after the initial graph fetch (the s3 read
		// is detected from the SQL body at deploy time; the missing-schedule edge is
		// synthesized client-side by the page's per-script annotation sweep), so a
		// cold-CI failure here points at that async timing, not a missing edge.
		const edges = [
			`Edge from asset:s3object:raw/orders/*.csv to script:${ingest}`,
			`Edge from script:${ingest} to asset:ducklake:${ordersTbl}`,
			`Edge from asset:ducklake:${ordersTbl} to script:${daily}`,
			`Edge from script:${daily} to asset:ducklake:${dailyTbl}`,
			`Edge from trigger:schedule:missing:${ingest} to script:${ingest}`
		]
		for (const name of edges) {
			// Edges are SVG <g> groups (no visible box of their own), so assert they
			// are rendered into the graph rather than in-viewport visible.
			await expect(page.getByRole('group', { name, exact: true }).first()).toBeAttached()
		}
	})
})
