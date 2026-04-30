// One-off repro for the asset-runs panel re-fetch flicker. Logs in, opens
// /pipeline/km, clicks the first s3 asset node, and counts list_jobs
// requests over a 10 s idle window. > 2 == reactivity bug still present.
import { chromium } from 'playwright'

const baseURL = process.env.BASE_URL ?? 'http://localhost:3000'
const folder = process.env.PIPELINE_FOLDER ?? 'km'

const browser = await chromium.launch({ headless: true })
const ctx = await browser.newContext()
const page = await ctx.newPage()

// Counters
const counters = { listJobs: 0, others: new Map() }
page.on('request', (req) => {
	const u = req.url()
	if (u.includes('/list_jobs') || u.includes('/concurrency_groups/list_jobs')) {
		counters.listJobs++
		console.log(`[${new Date().toISOString().slice(11, 23)}] LIST_JOBS #${counters.listJobs} ${u}`)
	}
})

// Console capture
page.on('console', (m) => {
	const t = m.type()
	if (t === 'error' || t === 'warning') console.log(`[console.${t}]`, m.text())
})

console.log('→ navigate to', baseURL)
await page.goto(baseURL)

// Login
await page.locator('input#email[type="email"]').fill('admin@windmill.dev')
await page.locator('input#password[type="password"]').fill('changeme')
await page.locator('button:has-text("Sign in")').click()
await page.waitForURL(/\/user\/(first-time|workspaces|$)|\/$/, { timeout: 15000 })

// First-time skip / workspace pick
if (page.url().includes('/user/first-time')) {
	await page.locator('button:has-text("Skip")').click()
}
if (page.url().includes('/user/workspaces')) {
	const ws = page.locator('text=Admins').first()
	await ws.waitFor({ state: 'visible' })
	await ws.click()
	await page.waitForURL(/\/$/)
}

console.log('→ navigate to /pipeline/' + folder)
await page.goto(`${baseURL}/pipeline/${folder}`)
await page.waitForLoadState('networkidle', { timeout: 10000 }).catch(() => {})

// Try to click the first asset node on the canvas. Asset nodes use
// AssetGenericIcon; a stable signal is the formatted-asset-path text rendered
// inside the node. Fallback: click any element with class containing 'asset'.
console.log('→ wait for canvas')
await page
	.locator('.svelte-flow__node-asset')
	.first()
	.waitFor({ state: 'visible', timeout: 15000 })

const beforeClick = counters.listJobs
console.log(`baseline list_jobs before asset click: ${beforeClick}`)

console.log('→ click first asset node')
await page.locator('.svelte-flow__node-asset').first().click()

// Wait briefly for the runs panel to mount and fire its initial fetch.
await page.waitForTimeout(1500)
console.log(`list_jobs after click + 1.5s: ${counters.listJobs}`)
const initial = counters.listJobs

// Idle window — no clicks, no input. Anything that fires here is the bug.
const idleSeconds = 10
console.log(`→ idle for ${idleSeconds}s`)
const tickStart = counters.listJobs
const start = Date.now()
while (Date.now() - start < idleSeconds * 1000) {
	await page.waitForTimeout(500)
}
const tickEnd = counters.listJobs
console.log(`list_jobs during idle: ${tickEnd - tickStart} (total: ${tickEnd})`)

// Try typing in the editor pane (if present) — this is what was triggering
// the flicker via graphWithDraft re-derivation in earlier reports. We only
// want to verify the fix protects against it.
const editor = page.locator('.monaco-editor').first()
if (await editor.isVisible().catch(() => false)) {
	console.log('→ focus + type a couple chars to test draft churn')
	await editor.click()
	const beforeType = counters.listJobs
	for (let i = 0; i < 6; i++) {
		await page.keyboard.type('x')
		await page.waitForTimeout(200)
	}
	console.log(`list_jobs while typing: ${counters.listJobs - beforeType}`)
}

console.log('=== final tally ===')
console.log({ total_list_jobs: counters.listJobs })

await browser.close()
