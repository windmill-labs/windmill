// Presentation-layer formatting helpers shared across the dashboard.
// Pure functions only — no React, no data fetching.

export function formatCurrency(amount: number, currency: string = 'USD'): string {
	if (!Number.isFinite(amount)) {
		return '—'
	}
	return new Intl.NumberFormat('en-US', {
		style: 'currency',
		currency,
		maximumFractionDigits: 0
	}).format(amount)
}

export function formatCurrencyPrecise(amount: number, currency: string = 'USD'): string {
	if (!Number.isFinite(amount)) {
		return '—'
	}
	return new Intl.NumberFormat('en-US', {
		style: 'currency',
		currency,
		minimumFractionDigits: 2,
		maximumFractionDigits: 2
	}).format(amount)
}

export function formatNumber(value: number): string {
	if (!Number.isFinite(value)) {
		return '—'
	}
	return new Intl.NumberFormat('en-US').format(value)
}

export function formatCompact(value: number): string {
	if (!Number.isFinite(value)) {
		return '—'
	}
	return new Intl.NumberFormat('en-US', {
		notation: 'compact',
		maximumFractionDigits: 1
	}).format(value)
}

export function formatPercent(ratio: number, digits: number = 1): string {
	if (!Number.isFinite(ratio)) {
		return '—'
	}
	return `${(ratio * 100).toFixed(digits)}%`
}

export function formatSignedPercent(ratio: number, digits: number = 1): string {
	const sign = ratio > 0 ? '+' : ''
	return `${sign}${formatPercent(ratio, digits)}`
}

export function formatDate(iso: string): string {
	const date = new Date(iso)
	if (Number.isNaN(date.getTime())) {
		return iso
	}
	return date.toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	})
}

export function formatDateShort(iso: string): string {
	const date = new Date(iso)
	if (Number.isNaN(date.getTime())) {
		return iso
	}
	return date.toLocaleDateString('en-US', {
		month: 'short',
		day: 'numeric'
	})
}

export function titleCase(value: string): string {
	return value
		.split(/[\s_-]+/)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
		.join(' ')
}

export function truncate(value: string, max: number = 32): string {
	if (value.length <= max) {
		return value
	}
	return `${value.slice(0, max - 1)}…`
}
