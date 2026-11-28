/**
 * Host-half regression test for `dsh-inline-image`.
 *
 * It pins the image route contract the deck depends on:
 *   - all seven bundled `ch-01.png … ch-07.png` samples are served as PNG;
 *   - the retry query (`&retry=N`) still resolves the same page;
 *   - HEAD returns headers without a body and unsupported methods answer 405;
 *   - paths outside the configured roots, absolute paths and unknown files 404;
 *   - unsupported media types 415 and over-limit files 413;
 *   - the health probe reports the Client bundle age the refresh hint reads.
 *
 *   node test/host.test.mjs      # 0 = all PASS, 1 = a FAIL
 */
import fs from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { apply } from '../index.js'

const TEST_DIR = dirname(fileURLToPath(import.meta.url))
const PLUGIN_DIR = join(TEST_DIR, '..')
const PUBLIC_DIR = join(PLUGIN_DIR, 'public')

let passes = 0
const failures = []
function expect(condition, message) {
  if (!condition) throw new Error(message)
}
async function test(name, body) {
  try {
    await body()
    passes += 1
    console.log('PASS  ' + name)
  } catch (error) {
    failures.push(name)
    console.error('FAIL  ' + name)
    console.error('      ' + (error instanceof Error ? error.message : String(error)))
  }
}

const routes = new Map()
const disposers = []
const ctx = {
  effect(run) {
    const dispose = run()
    if (typeof dispose === 'function') disposers.push(dispose)
    return () => {}
  },
  webServer: {
    register(route) {
      routes.set(route.path, route)
      return () => routes.delete(route.path)
    },
  },
}

const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'dsh-inline-image-test-'))
await fs.copyFile(join(PUBLIC_DIR, 'ch-01.png'), join(temporary, 'sample.png'))
await fs.writeFile(join(temporary, 'notes.txt'), 'not an image')

apply(ctx, { roots: [PUBLIC_DIR, temporary], maxBytes: 1024 * 1024 })

function respond(status, headers, body) {
  return { status, headers: headers || {}, body }
}

async function request(routePath, method, search) {
  const route = routes.get(routePath)
  expect(route !== undefined, 'route not registered: ' + routePath)
  const response = {
    status: 0,
    headers: {},
    body: null,
    writeHead(status, headers) {
      this.status = status
      this.headers = headers || {}
    },
    end(chunk) {
      this.body = chunk === undefined ? null : chunk
    },
  }
  await route.handler({ method, url: routePath + (search === undefined ? '' : '?' + search) }, response)
  return respond(response.status, response.headers, response.body)
}

function imagePath(file) {
  return '/dsh-inline-image/public?path=' + encodeURIComponent(file)
}

await test('both routes require an authenticated peer', async () => {
  expect(routes.get('/dsh-inline-image/public').auth === 'required', 'image route is not owner-authenticated')
  expect(routes.get('/dsh-inline-image/health').auth === 'required', 'health route is not owner-authenticated')
})

await test('all seven bundled samples are served as owner-private PNG', async () => {
  for (const file of ['ch-01.png', 'ch-02.png', 'ch-03.png', 'ch-04.png', 'ch-05.png', 'ch-06.png', 'ch-07.png']) {
    const result = await request('/dsh-inline-image/public', 'GET', 'path=' + encodeURIComponent(file))
    expect(result.status === 200, file + ' answered ' + result.status)
    expect(result.headers['content-type'] === 'image/png', file + ' content-type is ' + result.headers['content-type'])
    expect(result.headers['x-content-type-options'] === 'nosniff', file + ' is missing nosniff')
    expect(String(result.headers['cache-control']).includes('private'), file + ' is not private')
    const expected = (await fs.stat(join(PUBLIC_DIR, file))).size
    expect(result.body !== null && result.body.length === expected, file + ' body length differs from disk')
  }
})

await test('the retry query still serves the same page', async () => {
  const result = await request('/dsh-inline-image/public', 'GET', 'path=ch-01.png&retry=2')
  expect(result.status === 200, 'retry request answered ' + result.status)
  expect(result.headers['content-type'] === 'image/png', 'retry content-type changed')
})

await test('HEAD returns headers without a body', async () => {
  const result = await request('/dsh-inline-image/public', 'HEAD', 'path=ch-01.png')
  expect(result.status === 200, 'HEAD answered ' + result.status)
  expect(result.body === null, 'HEAD returned a body')
  expect(Number(result.headers['content-length']) > 0, 'HEAD is missing content-length')
})

await test('other methods answer 405 with an Allow header', async () => {
  const result = await request('/dsh-inline-image/public', 'POST', 'path=ch-01.png')
  expect(result.status === 405, 'POST answered ' + result.status)
  expect(String(result.headers.allow).includes('GET'), 'Allow header is missing GET')
})

await test('absolute paths and paths outside the roots answer 404', async () => {
  const absolute = await request('/dsh-inline-image/public', 'GET', 'path=' + encodeURIComponent(join(PUBLIC_DIR, 'ch-01.png')))
  expect(absolute.status === 404, 'absolute path answered ' + absolute.status)
  const traversal = await request('/dsh-inline-image/public', 'GET', 'path=' + encodeURIComponent('../../../../etc/passwd'))
  expect(traversal.status === 404, 'traversal answered ' + traversal.status)
  const missing = await request('/dsh-inline-image/public', 'GET', 'path=nope.png')
  expect(missing.status === 404, 'missing image answered ' + missing.status)
})

await test('a non-image file answers 415', async () => {
  const result = await request('/dsh-inline-image/public', 'GET', 'path=notes.txt')
  expect(result.status === 415, 'notes.txt answered ' + result.status)
  expect(String(result.headers['content-type']).includes('application/json'), 'error body is not JSON')
})

await test('the health route reports image resolution and the Client bundle age', async () => {
  const result = await request('/dsh-inline-image/health', 'GET', 'path=ch-01.png')
  expect(result.status === 200, 'health answered ' + result.status)
  const body = JSON.parse(result.body.toString('utf8'))
  expect(body.ok === true, 'health did not resolve ch-01.png')
  expect(body.mediaType === 'image/png', 'health media type changed')
  expect(Number(body.bytes) > 0, 'health lost the byte count')
  expect(Array.isArray(body.roots) && body.roots.some((root) => root === path.resolve(PUBLIC_DIR)), 'health roots lost the public directory')
  expect(typeof body.clientAgeMs === 'number' && body.clientAgeMs >= 0, 'health is missing clientAgeMs')
  const missing = await request('/dsh-inline-image/health', 'GET', 'path=nope.png')
  expect(JSON.parse(missing.body.toString('utf8')).ok === false, 'health must report a missing image')
})

await test('an over-limit image answers 413', async () => {
  const limited = new Map()
  const limitedCtx = {
    effect(run) { run(); return () => {} },
    webServer: { register(route) { limited.set(route.path, route); return () => {} } },
  }
  apply(limitedCtx, { roots: [PUBLIC_DIR], maxBytes: 10 })
  const route = limited.get('/dsh-inline-image/public')
  const response = { status: 0, headers: {}, body: null, writeHead(status, headers) { this.status = status; this.headers = headers || {} }, end(chunk) { this.body = chunk } }
  await route.handler({ method: 'GET', url: '/dsh-inline-image/public?path=ch-01.png' }, response)
  expect(response.status === 413, 'over-limit answered ' + response.status)
})

for (const dispose of disposers) dispose()
await fs.rm(temporary, { recursive: true, force: true })

console.log('')
console.log(passes + ' passed, ' + failures.length + ' failed')
if (failures.length > 0) process.exitCode = 1
