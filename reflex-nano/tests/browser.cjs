/* Starts both demos and tests real browser interactions. */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const {spawn} = require('node:child_process');
const {chromium} = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const base = path.resolve(__dirname,'..');
const servers = [], errors = [], checks = [];
const delay = ms => new Promise(r=>setTimeout(r,ms));
function start(command,args,env={}) {
  const child=spawn(command,args,{cwd:base,env:{...process.env,...env},stdio:['ignore','pipe','pipe']});
  child.stdout.on('data',data=>process.stdout.write(data));
  child.stderr.on('data',data=>process.stderr.write(data));
  servers.push(child); return child;
}
async function ready(url) {
  for(let i=0;i<100;i++) { try {if((await fetch(url)).ok)return;}catch{} await delay(100); }
  throw new Error('Server did not start: '+url);
}
async function idle(page) {
  await page.waitForFunction(()=>window.__NANO__?.ready && window.__NANO__.connection().connected && (!document.documentElement.dataset.nanoPending || document.documentElement.dataset.nanoPending==='0'));
}
async function text(page,selector,expected) {
  await page.waitForFunction(({selector,expected})=>document.querySelector(selector)?.textContent===expected,{selector,expected});
}
(async()=>{
  fs.mkdirSync(path.join(base,'docs'),{recursive:true});
  start(process.env.NANO_PYTHON || path.join(base,'.venv/bin/python'),['-m','reflex_nano','run','examples/python/dashboard.py','--port','3310']);
  start(process.env.NANO_RUST_DEMO || path.join(base,'target/release/examples/dashboard'),[],{PORT:'3311'});
  await Promise.all([ready('http://127.0.0.1:3310'),ready('http://127.0.0.1:3311')]);
  let options={headless:true,args:['--no-sandbox']};
  if(process.env.NANO_CHROMIUM_MODULE) {
    const binary=require(process.env.NANO_CHROMIUM_MODULE);
    options={headless:true,args:['--no-sandbox','--disable-dev-shm-usage','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader'],executablePath:await binary.executablePath()};
  }
  const browser=await chromium.launch(options);
  try {

    for(const [port,host] of [[3310,'Python bindings'],[3311,'Rust executable']]) {
      const context=await browser.newContext({viewport:{width:1280,height:1050}});
      const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
      const sockets=[],posts=[];
      page.on('websocket',ws=>sockets.push(ws.url()));
      page.on('request',r=>{if(r.method()==='POST'&&r.url().includes('/__nano/event'))posts.push(r.url());});
      await page.goto('http://127.0.0.1:'+port);await idle(page);await text(page,'#count','0');
      assert.equal(await page.evaluate(()=>window.__NANO__.renderer),process.env.NANO_RENDERER||'html');
      await page.getByRole('button',{name:'Increment',exact:true}).click();await text(page,'#count','1');
      await text(page,'#doubled','Computed double: 2');assert.equal(await page.locator('#positive').count(),1);
      await page.getByRole('button',{name:'Async +1',exact:true}).click();await text(page,'#count','2');
      await page.locator('#name').fill('F');await page.locator('#name').pressSequentially('arhan Nano',{delay:1});await idle(page);
      await text(page,'#greeting','Hello, Farhan Nano.');assert.equal(await page.evaluate(()=>document.activeElement.id),'name');
      await page.locator('#priority').selectOption('High');await idle(page);await text(page,'#priority-label','High priority');
      await page.locator('#task-input').fill('Ship native bindings');await page.getByRole('button',{name:'Add task',exact:true}).click();await idle(page);
      assert.equal(await page.locator('#tasks > div').count(),4);assert.equal(await page.locator('#task-input').inputValue(),'');
      await page.getByRole('checkbox',{name:'Complete Build the Rust runtime',exact:true}).check();await idle(page);await text(page,'#remaining','3 remaining');
      await page.getByRole('link',{name:'Expose native Python bindings',exact:true}).evaluate(n=>n.dataset.preserved='yes');
      await page.getByRole('button',{name:'Remove Build the Rust runtime',exact:true}).click();await idle(page);
      assert.equal(await page.getByRole('link',{name:'Expose native Python bindings',exact:true}).getAttribute('data-preserved'),'yes');
      await page.getByRole('link',{name:'Expose native Python bindings',exact:true}).click();await idle(page);await text(page,'#task-id','Task ID: 2');
      await page.goBack();await idle(page);await text(page,'#count','2');
      await page.getByRole('link',{name:'About',exact:true}).click();await idle(page);await text(page,'#about-count','Current counter: 2');
      await page.reload();await idle(page);await text(page,'#about-count','Current counter: 2');
      await page.getByRole('link',{name:'Back',exact:true}).click();await idle(page);
      await page.getByRole('button',{name:'Add task',exact:true}).click();await idle(page);
      assert.match(await page.locator('#nano-status').textContent(),/Write a task first/);assert.equal(await page.locator('#tasks > div').count(),3);
      await page.locator('#name').fill('</script><script>window.hacked=1</script>');await idle(page);await page.reload();await idle(page);
      assert.equal(await page.evaluate(()=>window.hacked),undefined);assert.match(await page.locator('#greeting').textContent(),/<script>/);
      assert.equal(await page.evaluate(()=>document.cookie.includes('nano_session')),false);
      await page.locator('#name').fill('Farhan Nano');await idle(page);
      const isolated=await browser.newContext();const other=await isolated.newPage();await other.goto('http://127.0.0.1:'+port);await idle(other);await text(other,'#count','0');await isolated.close();
      const fixtures=JSON.parse(fs.readFileSync(path.join(base,'tests/expression_cases.json'),'utf8'));
      for(const f of fixtures)assert.deepEqual(await page.evaluate(f=>window.__NANO__.evaluate(f.expression,f.state),f),f.expected,f.name);
      const tab=await context.newPage();tab.on('pageerror',e=>errors.push(e.message));
      await tab.goto('http://127.0.0.1:'+port+'/about');await idle(tab);await text(tab,'#about-count','Current counter: 2');
      await page.getByRole('button',{name:'Start background task',exact:true}).click();
      await page.getByRole('button',{name:'Stream +3',exact:true}).click();await text(page,'#count','5');
      await page.getByRole('button',{name:'Chain +5',exact:true}).click();await text(page,'#count','10');
      await text(page,'#progress','Progress: 5');await text(tab,'#about-count','Current counter: 10');
      assert.equal(new URL(tab.url()).pathname,'/about');
      const previous=sockets.length;await page.evaluate(()=>window.__NANO__.reconnect());
      await page.getByRole('button',{name:'Increment',exact:true}).click();await text(page,'#count','11');
      assert.ok(sockets.length>previous);await text(tab,'#about-count','Current counter: 11');
      await tab.goto('http://127.0.0.1:'+port);await idle(tab);
      await Promise.all([page,tab].map(p=>p.getByRole('button',{name:'Increment',exact:true}).click()));
      await text(page,'#count','13');await text(tab,'#count','13');await tab.close();
      assert.ok(sockets.length>=2&&sockets.every(url=>url.endsWith('/__nano/ws')));assert.deepEqual(posts,[]);
      if(process.env.NANO_RENDERER==='react') {
        const stats=await page.evaluate(()=>window.__NANO__.stats());
        assert.deepEqual(stats.recoverableErrors,[]);assert.deepEqual(stats.errors,[]);
      }
      if(port===3310)await page.screenshot({path:path.join(base,'docs/native-demo.png'),fullPage:true});
      await page.setViewportSize({width:390,height:844});
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'Mobile horizontal overflow');
      if(port===3310)await page.screenshot({path:path.join(base,'docs/native-demo-mobile.png'),fullPage:true});
      checks.push({host,passed:['WebSocket transport without HTTP event requests','sync and deferred Rust events','computed values and conditionals','controlled inputs, focus and selection','form validation and rollback','keyed list identity and nested mutation','dynamic routes, history and refresh','session isolation and escaping','expression parity','background push and streaming','server-side event chains','reconnect state retention','cross-route tab synchronization','concurrent same-session events','desktop and mobile layout']});
      await context.close();
    }
    assert.deepEqual(errors,[],'Browser JavaScript errors');
    const result={passed:true,renderer:process.env.NANO_RENDERER||'html',browser:browser.version(),checks,page_errors:errors};
    fs.writeFileSync(path.join(base,`docs/browser-${result.renderer}-results.json`),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
  } finally {await browser.close();}
})().catch(error=>{console.error(error.stack);process.exitCode=1;}).finally(()=>{for(const server of servers)server.kill('SIGTERM');});
