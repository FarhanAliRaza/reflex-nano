/* Correctness checks for optimizations, with no latency thresholds. */
const assert=require('node:assert/strict'), fs=require('node:fs'),path=require('node:path');
const {spawn,execFileSync}=require('node:child_process');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE_PATH||'playwright');
const root=path.resolve(__dirname,'..');
const renderer=process.env.NANO_RENDERER||'html';
const manifest=path.join(root,'tests/optimization-fixture.json');
execFileSync(process.env.NANO_PYTHON||path.join(root,'../.venv/bin/python'),
  [path.join(__dirname,'optimization_fixture.py'),manifest]);
const server=spawn(process.env.NANO_RUNNER||path.join(root,'target/release/examples/runner'),['--manifest',manifest],
  {env:{...process.env,PORT:'3358'},stdio:['ignore','pipe','pipe']});
let logs='';server.stdout.on('data',x=>logs+=x);server.stderr.on('data',x=>logs+=x);
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function idle(page) {
  await page.waitForFunction(()=>window.__NANO__?.ready&&window.__NANO__.connection().connected&&
    (!document.documentElement.dataset.nanoPending||document.documentElement.dataset.nanoPending==='0'));
}
async function count(page,value) {await page.waitForFunction(n=>document.getElementById('count')?.textContent===String(n)&&
  document.getElementById('doubled')?.textContent===String(n*2),value);await idle(page);}
(async()=>{
  for(let i=0;i<500;i++){try{if((await fetch('http://127.0.0.1:3358/__nano/style.css')).ok)break;}catch{}await delay(10);}
  const browser=await chromium.launch({headless:true,executablePath:process.env.NANO_CHROMIUM_PATH,
    args:['--no-sandbox','--disable-dev-shm-usage','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  const errors=[],checks=[];
  try {
    const context=await browser.newContext(),page=await context.newPage();
    page.on('pageerror',e=>errors.push(e.message));
    let release;const blocked=new Promise(r=>release=r);
    await context.route(renderer==='react'?'**/__nano/react/*/react.js':'**/__nano/asset/*/client.js',async route=>{await blocked;await route.continue();});
    await page.goto('http://127.0.0.1:3358/opt',{waitUntil:'commit'});
    await page.waitForFunction(()=>document.querySelectorAll('#rows > div').length===3);
    await page.evaluate(()=>{
      window.ssrElements=Array.from(document.querySelectorAll('#nano-root *'));
      const input=document.getElementById('early-input');input.focus();input.value='typed before hydration';
    });
    release();await idle(page);
    assert.equal(await page.evaluate(()=>window.ssrElements.every(n=>n.isConnected)),true);
    assert.equal(await page.locator('#early-input').inputValue(),'typed before hydration');
    assert.equal(await page.locator('#adjacent').textContent(),'ANanoZ');
    assert.equal(await page.locator('#constant').textContent(),'6');
    assert.equal(await page.locator('#groups').textContent(),'Group AxyGroup AGroup BzGroup B');
    const initial=await page.evaluate(()=>window.__NANO__.stats());
    if(renderer==='html')assert.equal(initial.delegatedListeners,8);
    else {assert.equal(initial.renderer,'react');assert.equal(initial.hydration,'hydrateRoot');assert.deepEqual(initial.recoverableErrors,[]);}
    checks.push('SSR element identity, adjacent/empty text, early input, constant folding, nested lexical scopes');
    await page.locator('#increment').click();await count(page,1);
    const update=await page.evaluate(()=>{const s=window.__NANO__.stats();return s.last||s;});
    if(renderer==='html')assert.ok(update.visited<25,JSON.stringify(update));assert.ok(update.skipped>0);
    assert.equal(await page.locator('#conditional').count(),1);
    checks.push('Dependency boundaries skip unrelated rows and activate empty conditional fragments');
    await page.evaluate(()=>{window.rows=Array.from(document.querySelectorAll('#rows > div'));
      window.edit=window.rows[0].querySelector('.edit');window.edit.focus();window.edit.setSelectionRange(1,3);
      document.getElementById('reorder').click();});await idle(page);
    assert.deepEqual(await page.locator('#rows > div').evaluateAll(nodes=>nodes.map(n=>Number(n.dataset.id))),[1,2,0]);
    assert.equal(await page.evaluate(()=>window.rows.every(n=>n.isConnected)),true);
    assert.deepEqual(await page.evaluate(()=>[document.activeElement===window.edit,
      window.edit.selectionStart,window.edit.selectionEnd]),[true,1,3]);
    await page.locator('#rows > div:first-child input[type=checkbox]').check();await idle(page);
    assert.equal(await page.evaluate(()=>window.__NANO__.snapshot().state.items[0].id),1);
    assert.equal(await page.evaluate(()=>window.__NANO__.snapshot().state.items[0].done),true);
    checks.push('Keyed reorder keeps DOM nodes, focus, selection and refreshes event index scopes');
    await page.locator('#empty').click();await idle(page);assert.equal(await page.locator('#rows > div').count(),0);
    await page.locator('#restore').click();await idle(page);assert.equal(await page.locator('#rows > div').count(),3);
    await page.locator('#nested').click();await idle(page);assert.equal(await page.locator('#groups').textContent(),'ChangedqChanged');
    checks.push('List removal/restoration and nested list replacement');
    await page.evaluate(()=>{for(let i=0;i<5;i++)document.getElementById('debounce').click();});await count(page,2);
    await delay(120);await count(page,2);
    await page.evaluate(()=>{for(let i=0;i<5;i++)document.getElementById('throttle').click();});await count(page,3);
    await delay(240);await count(page,3);
    await page.locator('#throttle').click();await count(page,4);
    await page.locator('#stop').click();await count(page,5);
    await page.locator('#bubble').click();await count(page,8);
    await page.evaluate(()=>{window.__NANO__.reconnect();document.getElementById('temporal').click();});await idle(page);await count(page,8);
    await page.locator('#debounced-name').fill('Final name');
    await page.waitForFunction(()=>document.getElementById('adjacent').textContent==='AFinal nameZ');
    checks.push('Debounce final arguments, leading throttle without trailing replay, temporal discard, propagation');
    await page.reload();await idle(page);await count(page,8);
    assert.equal(await page.locator('#adjacent').textContent(),'AFinal nameZ');
    checks.push('Rehydration retains authoritative state and event bindings');
    assert.deepEqual(errors,[]);
    const finalStats=await page.evaluate(()=>window.__NANO__.stats());
    if(renderer==='react') {assert.deepEqual(finalStats.recoverableErrors,[]);assert.deepEqual(finalStats.errors,[]);}
    const result={passed:true,renderer,browser:browser.version(),checks,initial_stats:initial,counter_update_stats:update,page_errors:errors};
    fs.writeFileSync(path.join(root,`docs/optimization-${renderer}-tests.json`),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
  } finally {await browser.close();}
})().catch(e=>{console.error(e.stack,logs);process.exitCode=1;}).finally(()=>server.kill('SIGTERM'));
