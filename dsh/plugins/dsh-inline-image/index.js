/**
 * `dsh-inline-image` Host half.
 *
 * A deliberately tiny, generic route for the `dsh-image` Markdown fence:
 *
 *   GET /dsh-inline-image/public?path=<workspace-relative-path>
 *
 * The route is owner-authenticated, serves only files under one of the
 * configured workspace roots, and whitelists image media types. The Client half
 * is purely presentational; it does not read local files itself.
 *
 * This is intentionally independent of any course/plugin-specific state.
 */

import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export const name = 'dsh-inline-image'

/** This plugin's own directory, so the bundled samples are portable. */
const HERE = path.dirname(fileURLToPath(import.meta.url))

/** The Client bundle; its age lets a running page notice a stale HMR copy. */
const CLIENT_FILE = path.join(HERE, 'client.js')

/** The route needs the Web server; keep the plugin inactive without one. */
export const inject = ['webServer']

const ROUTE = '/dsh-inline-image/public'
const DEFAULT_MAX_BYTES = 10 * 1024 * 1024

/** Extension → media type. SVG is served as an image, never as a document. */
const MEDIA = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  webp: 'image/webp',
  bmp: 'image/bmp',
  ico: 'image/x-icon',
  svg: 'image/svg+xml',
}

/** Normalize the optional `roots` config into absolute directories. */
function rootsOf(config) {
  const configured = Array.isArray(config?.roots) ? config.roots : []
  const roots = configured
    .filter((root) => typeof root === 'string' && root.trim() !== '')
    .map((root) => path.resolve(root.trim()))
  // Default: the bundled `public/` samples plus the Host working directory.
  // Never hard-code a machine-specific absolute path here.
  return roots.length > 0 ? roots : [path.join(HERE, 'public'), path.resolve(process.cwd())]
}

function sendJson(response, status, body) {
  const payload = Buffer.from(JSON.stringify(body), 'utf8')
  response.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'content-length': payload.length,
    'cache-control': 'no-store',
    'x-content-type-options': 'nosniff',
  })
  response.end(payload)
}

/** True when `child` is inside `parent` (both already absolute/real). */
function inside(parent, child) {
  const relative = path.relative(parent, child)
  return relative === '' || (!relative.startsWith('..' + path.sep) && relative !== '..' && !path.isAbsolute(relative))
}

/** Resolve one requested path under the configured roots, following no symlink outside. */
async function resolveImage(roots, requested) {
  if (typeof requested !== 'string' || requested === '' || requested.includes('\0')) return null
  if (path.isAbsolute(requested)) return null
  for (const root of roots) {
    const candidate = path.resolve(root, requested)
    const relative = path.relative(root, candidate)
    if (relative === '' || relative.startsWith('..' + path.sep) || relative === '..' || path.isAbsolute(relative)) continue
    let realRoot
    let realFile
    try {
      realRoot = await fs.realpath(root)
      realFile = await fs.realpath(candidate)
    } catch {
      continue
    }
    if (!inside(realRoot, realFile)) continue
    let stat
    try {
      stat = await fs.stat(realFile)
    } catch {
      continue
    }
    if (!stat.isFile()) continue
    const extension = path.extname(realFile).slice(1).toLowerCase()
    const mediaType = MEDIA[extension]
    if (mediaType === undefined) return { error: 'unsupported media type' }
    const maxBytes = Number.isFinite(Number(configMaxBytes)) ? Number(configMaxBytes) : DEFAULT_MAX_BYTES
    if (stat.size > maxBytes) return { error: 'image too large' }
    let bytes
    try {
      bytes = await fs.readFile(realFile)
    } catch {
      continue
    }
    return { bytes, mediaType, size: bytes.length, realFile }
  }
  return null
}

let configMaxBytes

export function apply(ctx, config = {}) {
  const roots = rootsOf(config)
  configMaxBytes = Number(config?.maxBytes)
  // Owner-authenticated: a same-origin browser <img> request carries the
  // session credential, so the route never leaks workspace images.
  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: ROUTE,
    auth: 'required',
    handler: async (request, response, peer) => {
      try {
        if (request.method !== 'GET' && request.method !== 'HEAD') {
          response.writeHead(405, { allow: 'GET, HEAD' })
          response.end()
          return
        }
        const url = new URL(String(request.url || '/'), 'http://dsh.invalid')
        const requested = url.searchParams.get('path')
        const image = await resolveImage(roots, requested)
        if (image === null || image.error !== undefined) {
          sendJson(response, image?.error === 'image too large' ? 413 : image?.error === 'unsupported media type' ? 415 : 404, {
            error: image?.error ?? 'image not found',
          })
          return
        }
        const headers = {
          'content-type': image.mediaType,
          'content-length': image.size,
          'cache-control': 'private, max-age=60',
          'content-disposition': 'inline',
          'x-content-type-options': 'nosniff',
        }
        response.writeHead(200, headers)
        if (request.method === 'HEAD') {
          response.end()
          return
        }
        response.end(image.bytes)
      } catch {
        sendJson(response, 500, { error: 'image read failed' })
      }
    },
  }), 'dsh-inline-image: raw workspace image route')

  /* Anonymous, content-free diagnostic used while wiring the route: it only
   * says whether a workspace-relative image would resolve. It never returns
   * bytes and exposes no file content. `clientAgeMs` is the age of this
   * plugin's Client bundle, which lets a page that is still running an older
   * HMR copy offer a refresh. */
  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: '/dsh-inline-image/health',
    auth: 'required',
    handler: async (request, response) => {
      try {
        const url = new URL(String(request.url || '/'), 'http://dsh.invalid')
        const requested = url.searchParams.get('path')
        const image = await resolveImage(roots, requested)
        let clientAgeMs
        try {
          const stat = await fs.stat(CLIENT_FILE)
          clientAgeMs = Math.max(0, Date.now() - stat.mtimeMs)
        } catch {
          /* The bundle is expected to exist; omit the age when it cannot be read. */
        }
        sendJson(response, 200, {
          ok: image !== null && image.error === undefined,
          error: image?.error ?? (image === null ? 'not found' : undefined),
          bytes: image?.size,
          mediaType: image?.mediaType,
          roots,
          clientAgeMs,
        })
      } catch {
        sendJson(response, 200, { ok: false, error: 'probe failed' })
      }
    },
  }), 'dsh-inline-image: health route')
}
