import { test, expect } from '@playwright/test';
import { createServer } from 'http';
import { readFileSync, existsSync, mkdirSync } from 'fs';
import { resolve, extname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const PORT = parseInt(process.env.PORT || '8080', 10);

// Serve built WASM files from the CMake output directory.
// Override with SERVE_DIR env var if deploying elsewhere.
const SERVE_DIR = process.env.SERVE_DIR ||
  resolve(__dirname, '../../../../build/examples/web/webrtc-loopback');

const MIME = {
  '.html': 'text/html',
  '.js':   'application/javascript',
  '.wasm': 'application/wasm',
  '.ts':   'application/typescript',
  '.css':  'text/css',
  '.png':  'image/png',
  '.svg':  'image/svg+xml',
};

const EVIDENCE_DIR = resolve(__dirname, 'test-evidence');

let server;

test.beforeAll(async () => {
  // Create evidence directory for screenshots
  if (!existsSync(EVIDENCE_DIR)) {
    mkdirSync(EVIDENCE_DIR, { recursive: true });
  }

  // Start static file server
  server = createServer((req, res) => {
    let filePath = resolve(SERVE_DIR, req.url === '/' ? 'index.html' : req.url.slice(1));
    if (!existsSync(filePath)) {
      res.writeHead(404);
      res.end('Not found');
      return;
    }
    const ext = extname(filePath).toLowerCase();
    res.writeHead(200, { 'Content-Type': MIME[ext] || 'application/octet-stream' });
    res.end(readFileSync(filePath));
  });

  await new Promise((resolveP) => server.listen(PORT, resolveP));
  console.log(`Test server running at http://localhost:${PORT}/`);
});

test.afterAll(() => {
  if (server) server.close();
});

// ─── Scenario 1: Basic P2P connection ───────────────────────────────────

test('P2P loopback — ICE Connected + frames', async ({ page }) => {
  test.setTimeout(180000);
  await page.goto(`http://localhost:${PORT}/`);

  // Wait for WASM module to initialize
  await expect(page.locator('#status')).toHaveText('Ready', { timeout: 30000 });

  // Click Start
  await page.click('#start-btn');

  // Wait for ICE to reach Connected state (text includes timestamp like "Connected (12.3s)")
  await expect(page.locator('#ice1')).toContainText('Connected', { timeout: 120000 });

  // Wait for receiver frames to start incrementing
  await expect(async () => {
    const count = parseInt(await page.textContent('#recv-count') || '0', 10);
    expect(count).toBeGreaterThan(0);
  }).toPass({ timeout: 60000 });

  await page.screenshot({
    path: resolve(EVIDENCE_DIR, 'p2p-connected.png'),
    fullPage: true,
  });
});

// ─── Scenario 2: Sender canvas shows SquarePattern ──────────────────────

test('Sender canvas shows SquarePattern (not blank)', async ({ page }) => {
  await page.goto(`http://localhost:${PORT}/`);
  await expect(page.locator('#status')).toHaveText('Ready', { timeout: 30000 });
  await page.click('#start-btn');
  await expect(page.locator('#ice1')).toContainText('Connected', { timeout: 30000 });

  // Wait a few frames to render
  await page.waitForTimeout(2000);

  // Sample pixel data from the sender canvas to verify it is not blank/all-green
  const pixelSample = await page.evaluate(() => {
    const canvas = /** @type {HTMLCanvasElement} */ (
      document.getElementById('send-canvas'));
    if (!canvas) return null;
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    // Sample a grid of 16 positions across the canvas
    const w = canvas.width;
    const h = canvas.height;
    const samples = [];
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        const px = Math.floor((x + 0.5) * w / 4);
        const py = Math.floor((y + 0.5) * h / 4);
        const pixel = ctx.getImageData(px, py, 1, 1).data;
        samples.push({ r: pixel[0], g: pixel[1], b: pixel[2], a: pixel[3] });
      }
    }
    return samples;
  });

  expect(pixelSample).not.toBeNull();
  // Verify at least one pixel has non-zero color (canvas is not pure black)
  const hasColor = pixelSample.some(p => p.r > 0 || p.g > 0 || p.b > 0);
  expect(hasColor).toBe(true);

  // Verify not all pixels are the same (actual pattern has variation)
  const unique = new Set(pixelSample.map(p => `${p.r},${p.g},${p.b}`));
  expect(unique.size).toBeGreaterThan(1);
});

// ─── Scenario 3: Auto-start via query parameter ─────────────────────────

test('Auto-start (?auto) connects without manual click', async ({ page }) => {
  await page.goto(`http://localhost:${PORT}/?auto`);

  // Auto-start triggers immediately after WASM init
  await expect(page.locator('#ice1')).toContainText('Connected', { timeout: 30000 });

  // Verify frames are being received
  await expect(async () => {
    const count = parseInt(await page.textContent('#recv-count') || '0', 10);
    expect(count).toBeGreaterThan(0);
  }).toPass({ timeout: 15000 });
});

// ─── Scenario 4: Stop + Restart ─────────────────────────────────────────

test('Stop + Restart resets counters and reconnects', async ({ page }) => {
  await page.goto(`http://localhost:${PORT}/`);
  await expect(page.locator('#status')).toHaveText('Ready', { timeout: 30000 });
  await page.click('#start-btn');
  await expect(page.locator('#ice1')).toContainText('Connected', { timeout: 30000 });

  // Wait for some frames
  await expect(async () => {
    const count = parseInt(await page.textContent('#recv-count') || '0', 10);
    expect(count).toBeGreaterThan(5);
  }).toPass({ timeout: 15000 });

  // Stop
  await page.click('#stop-btn');
  await expect(page.locator('#status')).toHaveText('Stopped', { timeout: 10000 });

  // Verify counters reset
  const sentAfterStop = await page.textContent('#sent-count');
  const recvAfterStop = await page.textContent('#recv-count');
  expect(sentAfterStop).toBe('0');
  expect(recvAfterStop).toBe('0');

  // Restart
  await page.click('#start-btn');
  await expect(page.locator('#ice1')).toContainText('Connected', { timeout: 30000 });

  // Verify frames start from 0 and increment
  await expect(async () => {
    const count = parseInt(await page.textContent('#recv-count') || '0', 10);
    expect(count).toBeGreaterThan(0);
  }).toPass({ timeout: 15000 });
});

// ─── Scenario 5: Stats panel populates ──────────────────────────────────

test('Stats panel populates after connection', async ({ page }) => {
  await page.goto(`http://localhost:${PORT}/`);
  await expect(page.locator('#status')).toHaveText('Ready', { timeout: 30000 });
  await page.click('#start-btn');
  await expect(page.locator('#ice1')).toContainText('Connected', { timeout: 30000 });

  // Wait for stats polling (interval is 2000ms — wait for at least 2 polls)
  await page.waitForTimeout(5000);

  const statsText = await page.textContent('#stats-text');
  expect(statsText).not.toBeNull();

  // Stats should not be the placeholder "—"
  expect(statsText.trim()).not.toBe('\u2014');
  expect(statsText.trim()).not.toBe('—');

  // Stats should contain numeric data (e.g., "fps:15  kbps:..." and "jitter:...")
  expect(statsText).toMatch(/\d/);
});
