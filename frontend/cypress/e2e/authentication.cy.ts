// TODO: Should correctly handle exceptions
Cypress.on('uncaught:exception', (err, runnable) => {
	// returning false here prevents Cypress from
	// failing the test
	return false
})

describe('Authentication', () => {
	it('can login using email and password', () => {
		cy.login('admin@windmill.dev', 'changeme')
		cy.contains('Select a workspace')
	})

	it('should redirect to login page if user is not logged in', () => {
		cy.visit(`/user/workspaces`)
		cy.url().should('include', '/user/login')
	})
})
