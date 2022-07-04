

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

test('can login', async ({ page }) => {
    await expect(page.locator('body')).toHaveText('Select a workspace')
})
