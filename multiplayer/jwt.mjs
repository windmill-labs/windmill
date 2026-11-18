/**
 * Verification of the Ed25519 JWTs the Windmill backend signs for the extra
 * services (multiplayer, language servers), against the key it publishes at
 * /api/debug/jwks.
 */

import crypto from 'node:crypto'

const WINDMILL_BASE_URL = process.env.WINDMILL_BASE_URL || process.env.BASE_INTERNAL_URL

let cachedPublicKey = null
let publicKeyFetchPromise = null
// The backend re-derives its key when the instance's JWT secret changes, so a
// signature the cached key rejects triggers a refetch, at most this often.
const KEY_REFRESH_INTERVAL_MS = 60_000
let lastKeyRefresh = 0

function base64urlDecode(str) {
  const padding = '='.repeat((4 - str.length % 4) % 4)
  const base64 = str.replace(/-/g, '+').replace(/_/g, '/') + padding
  const binary = atob(base64)
  return Uint8Array.from(binary, c => c.charCodeAt(0))
}

export async function getPublicKey() {
  if (cachedPublicKey) return cachedPublicKey
  if (publicKeyFetchPromise) return publicKeyFetchPromise

  if (!WINDMILL_BASE_URL) {
    console.warn(`[${new Date().toISOString()}] WINDMILL_BASE_URL not set - cannot fetch public key`)
    return null
  }

  publicKeyFetchPromise = (async () => {
    try {
      const jwksUrl = `${WINDMILL_BASE_URL.replace(/\/$/, '')}/api/debug/jwks`
      console.log(`[${new Date().toISOString()}] Fetching JWKS from ${jwksUrl}`)
      const response = await fetch(jwksUrl)
      if (!response.ok) {
        throw new Error(`Failed to fetch JWKS: ${response.status} ${response.statusText}`)
      }

      const jwks = await response.json()
      if (!jwks.keys || jwks.keys.length === 0) {
        throw new Error('No keys in JWKS')
      }

      const jwk = jwks.keys[0]
      if (jwk.kty !== 'OKP' || jwk.crv !== 'Ed25519') {
        throw new Error(`Unsupported key type: ${jwk.kty}/${jwk.crv}`)
      }

      const publicKeyBytes = base64urlDecode(jwk.x)
      const key = await crypto.subtle.importKey(
        'raw',
        publicKeyBytes,
        { name: 'Ed25519' },
        true,
        ['verify']
      )

      cachedPublicKey = key
      console.log(`[${new Date().toISOString()}] Successfully loaded Ed25519 public key from JWKS`)
      return key
    } catch (error) {
      console.error(`[${new Date().toISOString()}] Failed to fetch/parse JWKS: ${error}`)
      return null
    } finally {
      publicKeyFetchPromise = null
    }
  })()

  return publicKeyFetchPromise
}

/**
 * Verify the signature, expiry and purpose of a token.
 * Returns `{ claims }` when valid, otherwise `{ error }`, with `noKey` set when
 * the failure is only that the public key could not be obtained.
 */
export async function verifySignedToken(token, purpose, refreshed = false) {
  const publicKey = await getPublicKey()
  if (!publicKey) {
    return { error: 'Public key not available but signed requests are required. Set WINDMILL_BASE_URL.', noKey: true }
  }

  const parts = token.split('.')
  if (parts.length !== 3) {
    return { error: 'Invalid JWT format' }
  }

  const [headerB64, claimsB64, signatureB64] = parts

  try {
    const message = new TextEncoder().encode(`${headerB64}.${claimsB64}`)
    const signature = base64urlDecode(signatureB64)

    const isValid = await crypto.subtle.verify(
      { name: 'Ed25519' },
      publicKey,
      signature,
      message
    )

    if (!isValid) {
      if (!refreshed && Date.now() - lastKeyRefresh > KEY_REFRESH_INTERVAL_MS) {
        lastKeyRefresh = Date.now()
        cachedPublicKey = null
        return verifySignedToken(token, purpose, true)
      }
      return { error: 'Invalid JWT signature' }
    }

    const claims = JSON.parse(new TextDecoder().decode(base64urlDecode(claimsB64)))

    const now = Math.floor(Date.now() / 1000)
    if (!(now <= claims.exp)) {
      return { error: `Token expired: ${now - claims.exp} seconds ago` }
    }

    if (claims.purpose !== purpose) {
      return { error: `Invalid token purpose: ${claims.purpose}` }
    }

    return { claims }
  } catch (error) {
    return { error: `JWT verification error: ${error}` }
  }
}
