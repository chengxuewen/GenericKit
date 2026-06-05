import { chromium } from 'playwright';
import http from 'http';
import fs from 'fs';
import path from 'path';

const PORT = 8899;
const SERVE_DIR = path.resolve('../../../../build/examples/web/webrtc-loopback');
const TIMEOUT = 15000;

async function main() {
  const server = http.createServer((req, res) => {
    let filePath = path.join(SERVE_DIR, req.url === '/' ? '/index.html' : req.url);
    const ext = path.extname(filePath);
    const mime = {
      '.html': 'text/html',
      '.js': 'application/javascript',
      '.wasm': 'application/wasm',
      '.mjs': 'application/javascript',
    }[ext] || 'application/octet-stream';
    try {
      const data = fs.readFileSync(filePath);
      res.writeHead(200, {
        'Content-Type': mime,
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
      });
      res.end(data);
    } catch {
      res.writeHead(404);
      res.end();
    }
  });

  await new Promise(r => server.listen(PORT, r));
  console.log(`Server: http://localhost:${PORT}`);

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ bypassCSP: true });
  const page = await context.newPage();
  const logs = [];

  page.on('console', msg => {
    const text = msg.text();
    logs.push(text);
    if (text.startsWith('[STATS]')) console.log('  ' + text);
  });
  page.on('pageerror', err => logs.push('[PAGE_ERROR] ' + err.message));

  try {
    await page.goto(`http://localhost:${PORT}/index.html`, {
      waitUntil: 'domcontentloaded',
      timeout: 10000,
    });
    await page.waitForFunction(
      () => document.getElementById('status')?.textContent === 'Ready',
      { timeout: 15000 }
    );
    console.log('WASM ready, clicking Start...');
    await page.click('#start-btn');

    const startTime = Date.now();
    let passed = false;
    while (Date.now() - startTime < TIMEOUT) {
      await page.waitForTimeout(500);
      const recvText = await page.$eval('#recv-count', el => el.textContent);
      if (parseInt(recvText) > 0) { passed = true; break; }
    }

    if (passed) {
      console.log('PASS: receiver received frames');
      await browser.close(); server.close(); process.exit(0);
    } else {
      console.log('FAIL: receiver received 0 frames');
      console.log('\n--- Console Logs ---');
      logs.forEach(l => console.log('  ' + l));
      await browser.close(); server.close(); process.exit(1);
    }
  } catch(e) {
    console.log('ERROR:', e.message);
    console.log('\n--- Console Logs ---');
    logs.forEach(l => console.log('  ' + l));
    await browser.close(); server.close(); process.exit(1);
  }
}

main();
