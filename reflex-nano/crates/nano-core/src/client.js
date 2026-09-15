/* Shared native transport, expressions and optional direct DOM renderer. */
window.__NANO_START__ = (createRenderer) => {
  "use strict";
  const root = document.getElementById("nano-root");
  const status = document.getElementById("nano-status");
  let boot = JSON.parse(document.getElementById("nano-data").textContent);
  let alternate = null;
  let queue = Promise.resolve(), pending = 0;
  let socket, connected=false, reconnects=0, stopping=false;
  const requests=new Map(), waiters=new Set();
  const eventTimers=new Set();
  const currentPath=()=>location.pathname+location.search;
  const truth = v => Array.isArray(v) ? v.length > 0 : v && typeof v === "object" ? Object.keys(v).length > 0 : !!v;
  const text = v => v == null ? "" : typeof v === "string" ? v : JSON.stringify(v);
  const equal = (a, b) => {
    if (a === b) return true;
    if (!a || !b || typeof a !== "object" || typeof b !== "object") return false;
    if (Array.isArray(a) !== Array.isArray(b)) return false;
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every(k => Object.hasOwn(b, k) && equal(a[k], b[k]));
  };
  function evaluate(e, state, scope = {}) {
    switch (e.op) {
      case "literal": return e.value;
      case "state": return Object.hasOwn(state, e.name) ? state[e.name] : null;
      case "local": return Object.hasOwn(scope, e.name) ? scope[e.name] : null;
      case "get": {
        const value = evaluate(e.value, state, scope), key = evaluate(e.key, state, scope);
        return value != null && Object.hasOwn(Object(value), key) ? value[key] : null;
      }
      case "not": return !truth(evaluate(e.value, state, scope));
      case "trim": return text(evaluate(e.value,state,scope)).trim();
      case "object": return Object.fromEntries(Object.entries(e.fields).map(([k,v])=>[k,evaluate(v,state,scope)]));
      case "count": {const items=evaluate(e.items,state,scope);return Array.isArray(items)?items.filter(item=>truth(evaluate(e.predicate,state,{...scope,[e.name]:item}))).length:0;}
      case "length": {
        const value = evaluate(e.value, state, scope);
        return typeof value === "string" ? Array.from(value).length : Array.isArray(value) ? value.length : value && typeof value === "object" ? Object.keys(value).length : 0;
      }
      case "concat": return e.parts.map(p => text(evaluate(p, state, scope))).join("");
      case "select": return evaluate(truth(evaluate(e.condition,state,scope)) ? e.yes : e.no,state,scope);
      case "binary": {
        const a = evaluate(e.left, state, scope), b = evaluate(e.right, state, scope);
        switch (e.operator) {
          case "eq": return equal(a, b); case "ne": return !equal(a, b);
          case "and": return truth(a) && truth(b); case "or": return truth(a) || truth(b);
          case "add": if (typeof a === "string" || typeof b === "string") return text(a) + text(b);
        }
        if (typeof a !== "number" || typeof b !== "number") return null;
        let value;
        switch (e.operator) {
          case "add": value = a + b; break; case "sub": value = a - b; break;
          case "mul": value = a * b; break; case "div": value = b ? a / b : null; break;
          case "mod": value = b ? a % b : null; break;
          case "gt": return a > b; case "ge": return a >= b; case "lt": return a < b; case "le": return a <= b;
          default: return null;
        }
        return value == null || Number.isFinite(value) ? value : null;
      }
    }
    throw new Error("Unknown Nano expression");
  }
  const boolAttrs = new Set(["disabled","checked","selected","multiple","required","readonly","autofocus","hidden","open"]);
  const urlAttrs = new Set(["href","src","action","xlink:href","poster"]);
  function safeURL(value) {
    const s = text(value).trim();
    if (/[\u0000-\u001f\u007f-\u009f]/.test(s)) return false;
    const match = s.match(/^([^\/?#:]+):/);
    return !match || ["http","https","mailto","tel"].includes(match[1].toLowerCase());
  }
  function attrs(element, old, next, hydrating=false) {
    for (const key of new Set([...Object.keys(old), ...Object.keys(next)])) {
      let value = next[key];
      if (key.toLowerCase().startsWith("on") || ["srcdoc","style","innerhtml","formaction"].includes(key.toLowerCase())) continue;
      if (urlAttrs.has(key.toLowerCase()) && !safeURL(value)) value = null;
      const absent = value == null || (boolAttrs.has(key) && !truth(value));
      if (absent && element.hasAttribute(key)) {element.removeAttribute(key);counters.domWrites++;}
      else if (!absent && old[key] !== value) {element.setAttribute(key, text(value));counters.domWrites++;}
      if ((key === "checked" || key === "selected") && !(hydrating && element===document.activeElement)) element[key] = !absent;
      if (key === "value" && ["INPUT","TEXTAREA","SELECT"].includes(element.tagName)) {
        // An older input response must not overwrite text entered while it was in flight.
        if (!(element === document.activeElement && (pending > 1 || hydrating)) && element.value !== text(value)) element.value = text(value);
      }
    }
  }
  const emptyScope = Object.freeze({});
  let mounted = null;
  const statistics = {hydratedNodes:0,createdNodes:0,delegatedListeners:0,updates:0,last:{}};
  let counters = {};
  const expression = (value, scope) => evaluate(value, boot.state, scope);
  const propertyPlans = new WeakMap();
  function properties(values, scope) {
    let plan=propertyPlans.get(values);
    if(!plan) {
      const fixed={},dynamic=[];
      for(const [key,value]of Object.entries(values)) {
        if(value.op==='literal')Object.defineProperty(fixed,key,{value:value.value,enumerable:true});
        else dynamic.push([key,value]);
      }
      plan={fixed,dynamic};propertyPlans.set(values,plan);
    }
    if(!plan.dynamic.length)return plan.fixed;
    const next={...plan.fixed};for(const [key,value]of plan.dynamic)next[key]=expression(value,scope);return next;
  }
  function flatten(children) {const nodes=[];for(const child of children)for(const node of child.nodes)nodes.push(node);return nodes;}
  function place(parent, nodes) {
    const focused=document.activeElement;
    let cursor = parent.firstChild;
    for (const node of nodes) {
      if (node === cursor) cursor = cursor.nextSibling;
      else {parent.insertBefore(node,cursor); counters.domWrites++;}
    }
    while (cursor) {const next=cursor.nextSibling;cursor.remove();cursor=next;counters.domWrites++;}
    if(focused!==document.activeElement&&focused?.isConnected&&focused!==document.body)focused.focus({preventScroll:true});
  }
  function take(cursor, kind, tag, value) {
    let node = cursor?.next;
    const matches = node && (kind === 'text' ? node.nodeType === 3 : node.nodeType === 1 && node.localName === tag);
    if (matches && kind === 'text' && node.data !== value && node.data.startsWith(value)) {
      // HTML merges adjacent text nodes and omits empty nodes. Split only there.
      node.splitText(value.length); counters.domWrites++;
    }
    if (matches) {cursor.next=node.nextSibling;statistics.hydratedNodes++;}
    else {node=kind==='text'?document.createTextNode(''):document.createElement(tag);statistics.createdNodes++;}
    if (kind === 'text' && node.data !== value) {node.data=value;counters.domWrites++;}
    return {node, adopted:!!matches};
  }
  function rowKey(plan, scope, index) {
    return plan.body.kind === 'element' && plan.body.key ? text(expression(plan.body.key,scope)) : index;
  }
  function rowsFor(plan, scope) {
    const value=expression(plan.items,scope), items=Array.isArray(value)?value:[];
    const seen=new Set();
    return items.map((item,index) => {
      const local={...scope,[plan.name]:item,[plan.index]:index};
      const key=rowKey(plan,local,index);
      if(seen.has(key))throw new Error('Duplicate sibling key: '+key);
      seen.add(key);return {key,scope:local};
    });
  }
  function mount(plan, scope, cursor=null) {
    counters.visited++;
    const part={plan,scope,nodes:[],children:[]};
    switch(plan.kind) {
      case 'text': {
        const value=text(expression(plan.value,scope));
        part.element=take(cursor,'text',null,value).node;part.nodes=[part.element];break;
      }
      case 'element': {
        const {node:element,adopted}=take(cursor,'element',plan.tag);
        part.element=element;part.nodes=[element];part.events=plan.events;
        part.attrs=properties(plan.attrs,scope);part.styles=properties(plan.styles,scope);
        if(Object.keys(plan.events).length)element.__nanoNode=part;
        // textarea.value owns its raw-text content; preserve parser-created text.
        part.valueTextarea=plan.tag==='textarea'&&Object.hasOwn(plan.attrs,'value');
        if(!part.valueTextarea) {
          const childCursor=adopted?{next:element.firstChild}:null;
          const createdBefore=statistics.createdNodes;
          part.children=plan.children.map(p=>mount(p,scope,childCursor));
          // Matching SSR children already have the right parent and order.
          if(!adopted||statistics.createdNodes!==createdBefore||childCursor.next)
            place(element,flatten(part.children));
        }
        if(!adopted)for(const [k,v]of Object.entries(part.styles))element.style.setProperty(k,text(v));
        // Rust already emitted ordinary attributes. Hydration only synchronizes
        // form properties, which can differ from their serialized attributes.
        if(!adopted||['input','textarea','select','option'].includes(plan.tag))
          attrs(element,adopted?part.attrs:{},part.attrs,adopted);
        break;
      }
      case 'fragment': part.children=plan.children.map(p=>mount(p,scope,cursor));part.nodes=flatten(part.children);break;
      case 'when': {
        part.branch=truth(expression(plan.condition,scope))?'yes':'no';
        part.children=[mount(plan[part.branch],scope,cursor)];part.nodes=flatten(part.children);break;
      }
      case 'each': {
        part.rows=new Map();
        part.children=rowsFor(plan,scope).map(row=>{const child=mount(plan.body,row.scope,cursor);part.rows.set(row.key,child);return child;});
        part.nodes=flatten(part.children);break;
      }
      default:throw new Error('Unknown Nano node');
    }
    return part;
  }
  function update(part, scope, changed) {
    const plan=part.plan;
    if(plan._s && !plan._s.some(key=>changed.has(key)) && !plan._l.some(key=>part.scope[key]!==scope[key])) {
      counters.skipped++;return false;
    }
    counters.visited++;part.scope=scope;
    switch(plan.kind) {
      case 'text': {
        const value=text(expression(plan.value,scope));
        if(part.element.data!==value){part.element.data=value;counters.domWrites++;}return false;
      }
      case 'element': {
        let structure=false;
        for(const child of part.children)structure=update(child,scope,changed)||structure;
        if(structure)place(part.element,flatten(part.children));
        const nextAttrs=properties(plan.attrs,scope), nextStyles=properties(plan.styles,scope);
        for(const key of new Set([...Object.keys(part.styles),...Object.keys(nextStyles)])) {
          if(part.styles[key]!==nextStyles[key]){part.element.style.setProperty(key,text(nextStyles[key]));counters.domWrites++;}
        }
        attrs(part.element,part.attrs,nextAttrs);part.attrs=nextAttrs;part.styles=nextStyles;return false;
      }
      case 'fragment': {
        let structure=false;for(const child of part.children)structure=update(child,scope,changed)||structure;
        if(structure)part.nodes=flatten(part.children);return structure;
      }
      case 'when': {
        const branch=truth(expression(plan.condition,scope))?'yes':'no';
        if(branch!==part.branch) {part.branch=branch;part.children=[mount(plan[branch],scope)];part.nodes=flatten(part.children);return true;}
        const structure=update(part.children[0],scope,changed);
        if(structure)part.nodes=flatten(part.children);return structure;
      }
      case 'each': {
        const nextRows=new Map(), previous=part.children;
        let structure=false;
        const children=rowsFor(plan,scope).map((row,index)=>{
          let child=part.rows.get(row.key);
          if(child)structure=update(child,row.scope,changed)||structure;
          else {child=mount(plan.body,row.scope);structure=true;}
          if(previous[index]!==child)structure=true;
          nextRows.set(row.key,child);return child;
        });
        structure ||= children.length!==previous.length;
        part.rows=nextRows;part.children=children;
        if(structure)part.nodes=flatten(children);return structure;
      }
    }
  }
  // Preserve references of unchanged objects received inside a changed list field.
  function share(previous, next) {
    if(previous===next)return previous;
    if(!previous||!next||typeof previous!=='object'||typeof next!=='object'||Array.isArray(previous)!==Array.isArray(next))return next;
    const keys=Object.keys(next);let same=keys.length===Object.keys(previous).length;
    for(const key of keys){const value=share(previous[key],next[key]);if(value!==next[key])next[key]=value;if(!Object.hasOwn(previous,key)||value!==previous[key])same=false;}
    return same?previous:next;
  }
  function applyState(state) {
    const previous=boot.state, next=share(previous,state), changed=new Set();
    for(const key of new Set([...Object.keys(previous),...Object.keys(next)])) {
      if(previous[key]!==next[key]||Object.hasOwn(previous,key)!==Object.hasOwn(next,key))changed.add(key);
    }
    boot.state=next;if(changed.size)Promise.resolve(render(changed)).catch(error=>showError(error.message));
  }
  function render(changed=new Set(Object.keys(boot.state)), hydrate=false) {
    if(alternate) {
      document.title=boot.title;
      return alternate.render({...boot},hydrate).then(()=>window.dispatchEvent(new CustomEvent("nano:update",{detail:{version:boot.version}})));
    }
    const start=performance.now();counters={visited:0,skipped:0,domWrites:0};
    if(!mounted || mounted.plan!==boot.tree) {
      mounted=mount(boot.tree,emptyScope,hydrate?{next:root.firstChild}:null);place(root,mounted.nodes);
    } else if(update(mounted,emptyScope,changed))place(root,mounted.nodes);
    document.title=boot.title;statistics.updates++;
    statistics.last={...counters,milliseconds:performance.now()-start};
    window.dispatchEvent(new CustomEvent('nano:update',{detail:{version:boot.version}}));
  }
  function dispatchEvent(element, event) {
    const latest=element.__nanoNode, name=event.type, spec=latest?.events[name];
    fireEvent(latest,name,spec,element,event);
  }
  function fireEvent(latest,name,spec,element,event,projected=null) {
    if(!spec)return;
    if(spec.prevent_default||name==='submit')event?.preventDefault?.();
    if(spec.stop_propagation)event?.stopPropagation?.();
    if(spec.temporal&&!connected)return;
    latest.timing ||= new Map();
    const timing=latest.timing.get(name)||{last:-Infinity,timer:null};latest.timing.set(name,timing);
    if(spec.throttle_ms&&performance.now()-timing.last<spec.throttle_ms)return;
    let value=element?.type==='checkbox'?element.checked:element?.value;
    if(name==='submit'&&element?.tagName==='FORM') {
      value=Object.create(null);
      for(const [key,item]of new FormData(element,event.submitter)) {
        if(typeof item!=='string'){showError('File uploads are not supported yet');return;}
        if(Object.hasOwn(value,key))value[key]=Array.isArray(value[key])?[...value[key],item]:[value[key],item];else value[key]=item;
      }
    }
    const scope={...latest.scope,event:projected||{value,checked:element?.checked,key:event?.key}};
    const args=spec.args.map(arg=>evaluate(arg,boot.state,scope));
    const form=name==='submit'&&element?.getAttribute('data-nano-reset')==='true'?element:null;
    const run=()=>{
      if(latest.active===false||(element&&!element.isConnected)||stopping||(spec.temporal&&!connected))return;
      timing.last=performance.now();
      enqueue(async()=>{await send(spec.name,args);if(form)form.reset();});
    };
    if(spec.debounce_ms) {
      if(timing.timer!==null){clearTimeout(timing.timer);eventTimers.delete(timing.timer);}
      timing.timer=setTimeout(()=>{eventTimers.delete(timing.timer);timing.timer=null;run();},spec.debounce_ms);
      eventTimers.add(timing.timer);
    } else run();
  }
  for(const name of createRenderer?[]:['click','input','change','submit','keydown','keyup','focus','blur']) {
    const capture=name==='focus'||name==='blur';
    root.addEventListener(name,event=>{
      for(let node=event.target;node&&node!==root;node=node.parentNode) {
        dispatchEvent(node,event);
        if(capture||event.cancelBubble)break;
      }
    },capture);statistics.delegatedListeners++;
  }
  function showError(message) { status.textContent = message; status.dataset.error = "true"; }
  function enqueue(task) {
    pending++; document.documentElement.dataset.nanoPending = String(pending);
    queue = queue.then(task).catch(error => { showError(error.message); window.dispatchEvent(new CustomEvent("nano:error",{detail:error.message})); })
      .finally(() => { pending--; document.documentElement.dataset.nanoPending = String(pending); });
  }
  async function navigate(path, push = true) {
    const url = new URL(path, location.href);
    if (url.origin !== location.origin) { if (safeURL(path)) location.assign(url); return; }
    const response = await fetch("/__nano/page?path=" + encodeURIComponent(url.pathname + url.search));
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Navigation failed");
    if((data.renderer||'html')!==(boot.renderer||'html')) {location.assign(url.href);return;}
    for(const timer of eventTimers)clearTimeout(timer);eventTimers.clear();
    boot = data;
    if (push) history.pushState({},"",url.pathname + url.search + url.hash);
    await render(); window.scrollTo(0,0);
    if(connected) socket.send(JSON.stringify({type:'navigate',path:currentPath()}));
  }
  function connect() {
    if(stopping)return;
    const current=new WebSocket((location.protocol==='https:'?'wss:':'ws:')+'//'+location.host+'/__nano/ws','nano.v1');
    socket=current;connected=false;
    document.documentElement.dataset.nanoSocket='connecting';
    socket.onopen=()=>{if(current===socket&&!stopping)current.send(JSON.stringify({type:'hello',csrf:boot.csrf,path:currentPath()}));};
    socket.onmessage=({data:wire})=> {
      if(current!==socket)return;
      let data;try{data=JSON.parse(wire);}catch{showError('Invalid server message');socket.close();return;}
      if(data.type==='snapshot'||data.type==='update') {
        if(data.type==='snapshot'&&data.path!==currentPath()){current.send(JSON.stringify({type:'navigate',path:currentPath()}));return;}
        if(data.path===currentPath() && data.version>=boot.version) {
          const state=data.type==='snapshot'?data.state:{...boot.state,...data.delta};
          if(data.type==='update')for(const name of data.removed)delete state[name];
          boot.version=data.version;applyState(state);
        }
        if(data.type==='snapshot'&&!connected) {
          connected=true;reconnects=0;document.documentElement.dataset.nanoSocket='connected';
          for(const waiter of waiters)waiter();waiters.clear();
          window.dispatchEvent(new CustomEvent('nano:connected'));
        }
      } else if(data.type==='ack') {
        const request=requests.get(data.id);if(!request)return;
        requests.delete(data.id);clearTimeout(request.timer);
        data.ok?request.resolve(data):request.reject(new Error(data.error||'Event failed'));
      } else if(data.type==='error'||data.type==='background_error') {
        showError(data.error);window.dispatchEvent(new CustomEvent('nano:error',{detail:data.error}));
      }
    };
    socket.onclose=()=> {
      if(current!==socket)return;
      connected=false;document.documentElement.dataset.nanoSocket='disconnected';
      for(const request of requests.values()) {
        clearTimeout(request.timer);request.reject(new Error('Connection lost. The event outcome is unknown; state will resync. It was not automatically replayed.'));
      }
      requests.clear();
      if(!stopping)setTimeout(()=>{if(current===socket)connect();},Math.min(5000,100*2**Math.min(reconnects++,6)));
    };
    socket.onerror=()=>{}; // onclose owns reconnect and pending-event cleanup.
  }
  function waitForSocket() {
    if(connected&&socket.readyState===WebSocket.OPEN)return Promise.resolve();
    return new Promise((resolve,reject)=>{
      const done=()=>{clearTimeout(timer);resolve();};
      const timer=setTimeout(()=>{waiters.delete(done);reject(new Error('WebSocket unavailable; reload to renew your session.'));},30000);
      waiters.add(done);
    });
  }
  async function send(name,args,depth=0) {
    if(depth>32)throw new Error('Event chain exceeds 32 steps');
    status.textContent = ""; delete status.dataset.error;
    await waitForSocket();
    const id=crypto.randomUUID();
    const data=await new Promise((resolve,reject)=> {
      const timer=setTimeout(()=>{requests.delete(id);reject(new Error('Event timed out; outcome unknown. It was not replayed.'));},60000);
      requests.set(id,{resolve,reject,timer});
      try{socket.send(JSON.stringify({type:'event',id,name,args,path:currentPath()}));}
      catch(error){requests.delete(id);clearTimeout(timer);reject(error);}
    });
    for (const effect of data.effects) {
      if (effect.kind === "redirect") await navigate(effect.path);
      else if (effect.kind === "alert") window.alert(effect.message);
      else if (effect.kind === "event") await send(effect.name,effect.args,depth+1);
    }
  }
  document.addEventListener("click",event => {
    const link = event.target.closest?.("a[href]");
    if (!link || event.defaultPrevented || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey || link.target || link.hasAttribute("download")) return;
    const url = new URL(link.href);
    if (url.origin !== location.origin || (url.pathname === location.pathname && url.hash)) return;
    event.preventDefault(); enqueue(() => navigate(url.href));
  });
  window.addEventListener("popstate",() => enqueue(() => navigate(location.href,false)));
  // Useful for integration tools; data only, no backend execution capability.
  window.__NANO__ = {snapshot:() => structuredClone(boot), evaluate, ready:false, stats:()=>structuredClone(alternate?alternate.stats():statistics),renderer:boot.renderer||"html",
    connection:()=>({transport:'websocket',connected}),reconnect:()=>{connected=false;socket.close();}};
  window.addEventListener('pagehide',()=>{
    stopping=true;connected=false;
    for(const request of requests.values()){clearTimeout(request.timer);request.reject(new Error('Page left during event; outcome unknown.'));}
    requests.clear();socket?.close();
    for(const timer of eventTimers)clearTimeout(timer);eventTimers.clear();
  });
  window.addEventListener('pageshow',event=>{if(event.persisted){stopping=false;connect();}});
  if(createRenderer) alternate=createRenderer({root, evaluate, text, truth, safeURL,
    fireEvent, pending:()=>pending, connected:()=>connected,
    dispose:owner=>{owner.active=false;for(const item of owner.timing?.values()||[])if(item.timer!==null){clearTimeout(item.timer);eventTimers.delete(item.timer);}},
    showError, getBoot:()=>boot});
  connect();
  Promise.resolve(render(new Set(Object.keys(boot.state)),true)).then(()=>{
    window.__NANO__.ready=true;window.dispatchEvent(new CustomEvent('nano:hydrated'));
  }).catch(error=>{showError(error.message);window.dispatchEvent(new CustomEvent('nano:error',{detail:error.message}));});
};
if((JSON.parse(document.getElementById('nano-data').textContent).renderer||'html')==='html')window.__NANO_START__();
