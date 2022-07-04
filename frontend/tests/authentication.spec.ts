

// describe('Authentication', () => {
//     it('can login using email and password', () => {
//         cy.login('admin@windmill.dev', 'changeme')
//         cy.contains('Select a workspace')
//     })

//     it('should redirect to login page if user is not logged in', () => {
//         cy.visit(`/user/workspaces`)
//         cy.url().should('include', '/user/login')
//     })
// })

import { test, expect } from '@playwright/test'


test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:8000/user/login')
    await page.locator('text=login without third-party').click()
    await page.locator('#email').fill('admin@windmill.dev')
    await page.locator('input[type="password"]').fill('changeme')
    await page.locator('text="Login"').click()

    await page.context().storageState({ path: 'storageState.json' })
})

test('can login', async ({ page }) => {
    await expect(page.locator('text=Select a workspace')).toBeVisible()
})
