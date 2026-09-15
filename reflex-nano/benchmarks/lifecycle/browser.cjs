/* One server and one browser at a time. All timestamps are from the renderer. */
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const {spawn} = require('node:child_process');
const {chromium} = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const here = __dirname, root = path.resolve(here, '../..');
const rows = Number(process.env.NANO_BENCH_ROWS || 100);
const kind = process.env.BENCH_KIND || 'nano';
const repetitions = Number(process.env.BENCH_REPETITIONS || 2);
const python = process.env.NANO_PYTHON || path.resolve(root, '../.venv/bin/python');
const manifest = path.join(here, `nano-${rows}.json`);
const command = ['nano', 'nano_react', 'nano_before'].includes(kind) ? [path.join(root,
  kind === 'nano_before' ? 'benchmarks/baselines/0.2.0/nano' : 'dist/nano'), '--manifest', manifest] :
  [python, kind === 'reflex' ? 'reflex_server.py' : 'nano_fixture.py'];
const start = performance.now();
const server = spawn(command[0], command.slice(1), {cwd: here, env: {...process.env,
  REFLEX_ENV_MODE: 'prod', NANO_RENDERER: kind.includes('react')?'react':'html', NANO_BENCH_PORT: '3355', NANO_MANIFEST: manifest}, stdio: ['ignore', 'pipe', 'pipe']});
let logs = ''; server.stdout.on('data', x => logs += x); server.stderr.on('data', x => logs += x);
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function ready() {
  for (let n = 0; n < 3000; n++) {
    if (server.exitCode !== null) throw new Error(logs);
    try {
      const r = await fetch('http://127.0.0.1:' + (kind === 'reflex' ? '3356/ping' : '3355/__nano/style.css'));
      await r.arrayBuffer();
      if (r.ok) return performance.now() - start;
    } catch {}
    await sleep(5);
  }
  throw new Error('Startup timeout: ' + logs);
}
(async () => {
  const startup_ms = await ready();
  const options = {headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage',
    '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader']};
  if (process.env.NANO_CHROMIUM_PATH) options.executablePath = process.env.NANO_CHROMIUM_PATH;
  const browser = await chromium.launch(options);
  const result = {kind, rows, startup_ms, browser: browser.version(), navigations: [], errors: []};
  try {
    for (let repetition = 0; repetition < repetitions; repetition++) {
      const context = await browser.newContext({viewport: {width: 1280, height: 900}});
      const page = await context.newPage();
      page.setDefaultTimeout(45000);
      page.on('pageerror', e => result.errors.push(e.message));
      page.on('requestfailed', r => result.errors.push(r.url() + ': ' + r.failure()?.errorText));
      const probe = fs.readFileSync(path.join(here, 'probe.js'), 'utf8');
      // sessionStorage survives reload; each fresh browser context starts at zero.
      await page.addInitScript({content: `(${probe})({rows:${rows},kind:${JSON.stringify(kind)},expectedCount:Number(sessionStorage.getItem('bench-count')||0)})`});
      for (const phase of ['initial', 'first_reload']) {
        const response = phase === 'initial' ? await page.goto('http://127.0.0.1:3355/bench/', {waitUntil: 'commit'}) :
          await page.reload({waitUntil: 'commit'});
        assert.equal(response.status(), 200, 'Document response');
        try {await page.waitForFunction(() => window.__lifecycle?.marks.paint_ready_ms !== undefined);}
        catch (e) {
          const detail = await page.evaluate(() => ({marks:window.__lifecycle?.marks,
            state: {...window.__lifecycle?.state, items:window.__lifecycle?.state.items?.length},
            html:document.body.innerHTML.slice(0, 2500),errors:window.__lifecycle?.errors}));
          throw new Error(JSON.stringify({error: e.message, detail, pageErrors:result.errors, logs}));
        }
        const record = await page.evaluate(() => {
          const m = window.__lifecycle;
          const nav = performance.getEntriesByType('navigation')[0];
          const resources = performance.getEntriesByType('resource').filter(r => /^https?:/.test(r.name));
          return {marks: m.marks, websocket_in_bytes: m.websocket_in_bytes,
            websocket_out_bytes: m.websocket_out_bytes, incoming_frames: m.incoming_frames,
            errors: m.errors, document: nav.toJSON(), resources: resources.map(r => r.toJSON()),
            http_transfer_bytes: nav.transferSize + resources.reduce((s,r) => s+r.transferSize,0),
            http_decoded_bytes: nav.decodedBodySize + resources.reduce((s,r) => s+r.decodedBodySize,0),
            row_count: document.querySelectorAll('#items > .item').length,
            dom_nodes: document.getElementsByTagName('*').length,
            native_hydration_stats: window.__NANO__?.stats?.()};
        });
        const next = phase === 'initial' ? 1 : 2;
        record.first_event_ms = await page.evaluate(next => window.__lifecycle.event('#increment', 'count', next, 'doubled', next*2), next);
        record.native_event_stats = await page.evaluate(() => window.__NANO__?.stats?.());
        await page.evaluate(next => sessionStorage.setItem('bench-count', String(next)), next);
        if (phase === 'first_reload') {
          await page.evaluate(() => document.querySelector('#items > .item:last-child button').click());
          await page.waitForFunction(rows => document.querySelector('#items > .item:last-child .item-done')?.textContent === 'true' &&
            document.getElementById('remaining')?.textContent === String(rows-1), rows);
          record.last_row_event_verified = true;
        }
        assert.deepEqual(record.errors, []);
        if(kind.includes('react')) {
          assert.equal(record.native_hydration_stats.renderer,'react');
          assert.equal(record.native_hydration_stats.hydration,'hydrateRoot');
          assert.deepEqual(record.native_hydration_stats.recoverableErrors,[]);
          assert.deepEqual(record.native_hydration_stats.errors,[]);
        }
        result.navigations.push({phase, repetition, ...record});
        console.log(JSON.stringify({kind, rows, repetition, phase, ready_ms: record.marks.ready_ms, event_ms: record.first_event_ms}));
      }
      await context.close();
    }
    assert.deepEqual(result.errors, []);
    fs.writeFileSync(process.env.BENCH_OUTPUT || path.join(here, 'browser-pilot.json'), JSON.stringify(result, null, 2));
  } finally {await browser.close();}
})().catch(e => {console.error(e.stack); process.exitCode = 1;}).finally(async () => {
  server.kill('SIGTERM');
  for (let n=0; n<100 && server.exitCode === null; n++) await sleep(20);
  if(server.exitCode === null) server.kill('SIGKILL');
});
