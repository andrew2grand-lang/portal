import { describe, expect, it } from 'vitest'
// @ts-expect-error: a plain .mjs script without types
import { plan } from './ci-signing.mjs'

const apple = {
  S_APPLE_CERTIFICATE: 'MIIK…base64',
  S_APPLE_CERTIFICATE_PASSWORD: 'p12-password',
  S_APPLE_SIGNING_IDENTITY: 'Developer ID Application: Sam Example (ABCDE12345)',
}

describe('CI code signing', () => {
  it('changes nothing without secrets (ad-hoc on macOS, unsigned on Windows)', () => {
    expect(plan({}, 'darwin')).toMatchObject({ signed: null, config: null, vars: {} })
    expect(plan({ S_APPLE_CERTIFICATE: '' }, 'darwin').signed).toBeNull()
    expect(plan({}, 'win32')).toMatchObject({ signed: null, config: null, vars: {} })
  })
  it('signs with Developer ID and notarises with an API key when those secrets exist', () => {
    const p = plan({ ...apple, S_APPLE_API_ISSUER: 'iss', S_APPLE_API_KEY: 'KEY123', S_APPLE_API_KEY_P8: '-----BEGIN PRIVATE KEY-----' }, 'darwin')
    expect(p.signed).toBe('developer-id')
    expect(p.config).toEqual({ bundle: { macOS: { signingIdentity: apple.S_APPLE_SIGNING_IDENTITY } } })
    expect(p.vars).toMatchObject({ APPLE_CERTIFICATE: apple.S_APPLE_CERTIFICATE, APPLE_API_ISSUER: 'iss', APPLE_API_KEY: 'KEY123' })
    expect(p.files).toEqual([{ name: 'AuthKey_KEY123.p8', content: '-----BEGIN PRIVATE KEY-----', env: 'APPLE_API_KEY_PATH' }])
    expect(p.what).toContain('notarised')
  })
  it('notarises with an Apple ID instead, or says it is not notarised', () => {
    const id = plan({ ...apple, S_APPLE_ID: 'm@example.com', S_APPLE_PASSWORD: 'app-pw', S_APPLE_TEAM_ID: 'ABCDE12345' }, 'darwin')
    expect(id.vars).toMatchObject({ APPLE_ID: 'm@example.com', APPLE_TEAM_ID: 'ABCDE12345' })
    expect(plan(apple, 'darwin').what).toContain('not notarised')
  })
  it('half the Apple secrets is not enough to sign', () => {
    expect(plan({ S_APPLE_CERTIFICATE: 'x', S_APPLE_CERTIFICATE_PASSWORD: 'y' }, 'darwin').signed).toBeNull()
  })
  it('uses Azure Artifact Signing first on Windows', () => {
    const p = plan(
      {
        S_AZURE_CLIENT_ID: 'c',
        S_AZURE_CLIENT_SECRET: 's',
        S_AZURE_TENANT_ID: 't',
        S_AZURE_SIGNING_ENDPOINT: 'https://weu.codesigning.azure.net',
        S_AZURE_SIGNING_ACCOUNT: 'seamdrop',
        S_AZURE_CERT_PROFILE: 'public',
        S_WINDOWS_CERTIFICATE: 'pfx',
        S_WINDOWS_CERTIFICATE_PASSWORD: 'pw',
      },
      'win32',
    )
    expect(p.signed).toBe('azure')
    expect(p.config).toEqual({
      bundle: {
        windows: { signCommand: 'artifact-signing-cli -e https://weu.codesigning.azure.net -a seamdrop -c public -d SeamDrop %1' },
      },
    })
    expect(p.vars).toEqual({ AZURE_CLIENT_ID: 'c', AZURE_CLIENT_SECRET: 's', AZURE_TENANT_ID: 't' })
  })
  it('falls back to a sign command, then to a .pfx certificate', () => {
    expect(plan({ S_WINDOWS_SIGN_COMMAND: 'signer sign %1' }, 'win32').config).toEqual({
      bundle: { windows: { signCommand: 'signer sign %1' } },
    })
    const pfx = plan({ S_WINDOWS_CERTIFICATE: 'cGZ4', S_WINDOWS_CERTIFICATE_PASSWORD: 'pw' }, 'win32')
    expect(pfx.signed).toBe('certificate')
    expect(pfx.pfx).toEqual({ base64: 'cGZ4', password: 'pw', timestampUrl: 'http://timestamp.digicert.com' })
  })
})
