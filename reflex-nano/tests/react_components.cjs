const assert=require('node:assert/strict'),path=require('node:path'),fs=require('node:fs');
const {spawn,execFileSync}=require('node:child_process');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE_PATH||'playwright');
const root=path.resolve(__dirname,'..'),python=process.env.NANO_PYTHON||path.resolve(root,'../.venv/bin/python');
const manifest=path.join(root,'examples/native/react-components.json');
execFileSync(python,[path.join(root,'examples/python/react_components.py'),manifest]);
const server=spawn(process.env.NANO_RUNNER||path.join(root,'target/release/examples/runner'),['--manifest',manifest],
  {env:{...process.env,PORT:'3359',NANO_RENDERER:'react'},stdio:['ignore','pipe','pipe']});
let logs='';server.stdout.on('data',s=>logs+=s);server.stderr.on('data',s=>logs+=s);
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const idle=page=>page.waitForFunction(()=>window.__NANO__?.ready&&window.__NANO__.connection().connected&&
  (!document.documentElement.dataset.nanoPending||document.documentElement.dataset.nanoPending==='0'));
(async()=>{
  for(let i=0;i<500;i++){try{if((await fetch('http://127.0.0.1:3359/')).ok)break;}catch{}await pause(10);}
  const browser=await chromium.launch({headless:true,executablePath:process.env.NANO_CHROMIUM_PATH,
    args:['--no-sandbox','--disable-dev-shm-usage','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  const errors=[],requests=[];
  try {
    const page=await browser.newPage();page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>requests.push({url:r.url(),method:r.method()}));
    await page.goto('http://127.0.0.1:3359/');await idle(page);
    assert.equal(await page.evaluate(()=>window.__NANO__.renderer),'html');
    assert.ok(requests.every(r=>!r.url.includes('/__nano/react/')));
    await page.locator('#increment').click();await page.waitForFunction(()=>document.getElementById('count').textContent==='1');
    await page.locator('#to-widgets').click();await idle(page);
    await page.waitForSelector('#native-switch');
    assert.equal(await page.evaluate(()=>window.__NANO__.renderer),'react');
    assert.equal(await page.locator('#count').textContent(),'1');
    await page.locator('#increment').click();await page.waitForFunction(()=>document.getElementById('count').textContent==='2');
    await page.locator('#name').fill('Farhan React');await page.waitForFunction(()=>document.getElementById('name-value').textContent==='Farhan React');
    await page.locator('#native-switch').click();await page.waitForFunction(()=>document.getElementById('checked-value').textContent==='on');
    await page.locator('#choice').click();await page.getByRole('option',{name:'Second'}).click();
    await page.waitForFunction(()=>document.getElementById('choice-value').textContent==='two');
    await page.locator('#open-dialog').click();await page.getByRole('dialog').waitFor();
    assert.equal(await page.evaluate(()=>window.__NANO__.snapshot().state.open),true);
    assert.equal(await page.locator('#portal-increment').evaluate(n=>document.getElementById('nano-root').contains(n)),false);
    await page.locator('#portal-increment').click();await page.waitForFunction(()=>document.getElementById('count').textContent==='3');
    await page.locator('#close-dialog').click();await page.getByRole('dialog').waitFor({state:'hidden'});
    await page.locator('#custom-counter').click();await page.waitForFunction(()=>document.getElementById('count').textContent==='4');
    assert.match(await page.locator('#custom-counter').textContent(),/local clicks: 1/);
    await page.locator('#increment').click();await page.waitForFunction(()=>document.getElementById('count').textContent==='5');
    assert.match(await page.locator('#custom-counter').textContent(),/Rust: 5; local clicks: 1/);
    await page.reload();await idle(page);await page.waitForSelector('#native-switch');
    assert.equal(await page.locator('#count').textContent(),'5');assert.equal(await page.locator('#name').inputValue(),'Farhan React');
    const stats=await page.evaluate(()=>window.__NANO__.stats());
    assert.deepEqual(stats.errors,[]);assert.deepEqual(stats.recoverableErrors,[]);
    await page.locator('#to-html').click();await idle(page);
    assert.equal(await page.evaluate(()=>window.__NANO__.renderer),'html');assert.equal(await page.locator('#count').textContent(),'5');
    assert.deepEqual(errors,[]);assert.ok(!requests.some(r=>r.method==='POST'&&r.url.includes('/event')));
    const result={passed:true,react:stats.reactVersion,browser:browser.version(),checks:[
      'HTML route downloads no React','Renderer transition retains Rust session',
      'Radix Button, TextField, controlled Switch and Select',
      'Controlled Dialog and Rust callback from portal outside root',
      'Custom React hooks survive Rust state updates','Reload retains native state; native HTML fallback route'],errors,stats};
    fs.writeFileSync(path.join(root,'docs/react-components-tests.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
  } finally {await browser.close();}
})().catch(e=>{console.error(e.stack,logs);process.exitCode=1;}).finally(()=>server.kill('SIGTERM'));
