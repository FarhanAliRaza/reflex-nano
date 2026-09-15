import React, {memo, useRef, useState, useLayoutEffect, useContext, createContext} from 'react';
import {useComposedRefs} from '@radix-ui/react-compose-refs';
import {createRoot, hydrateRoot} from 'react-dom/client';
import '../crates/nano-core/src/client.js';

// These contexts expose native snapshots and event dispatch to registered React
// components. They deliberately carry no Python State objects or Socket.IO state.
export const NanoStateContext=createContext(null);
export const NanoEventContext=createContext(null);
export const useNanoState=()=>useContext(NanoStateContext);
export const useNanoEvents=()=>useContext(NanoEventContext);
const h=React.createElement, emptyScope=Object.freeze({});
const hasComponents=plan=>plan.kind==='component'||(plan.children||[]).some(hasComponents)||
  (plan.body&&hasComponents(plan.body))||(plan.yes&&(hasComponents(plan.yes)||hasComponents(plan.no)));
let registryPromise;
const loadRegistry=()=>registryPromise||=(import('./components.js').then(module=>module.registry));

function createRenderer(api) {
  let reactRoot, registry={}, initialized=false, firstCommit=null, resolveFirst, current,
    sequence=0, pendingCommit=new Map();
  const stats={renderer:'react',reactVersion:React.version,hydration:'pending',commits:0,
    visited:0,skipped:0,recoverableErrors:[],errors:[]};
  function fail(error) {
    stats.errors.push(error.message);api.showError(error.message);
    resolveFirst?.();
    for(const waiter of pendingCommit.values())waiter.reject(error);pendingCommit.clear();
  }
  class ErrorBoundary extends React.Component {
    state={error:null};
    static getDerivedStateFromError(error){return {error};}
    componentDidCatch(error){fail(error);}
    render(){return this.state.error?h('p',{role:'alert'},this.state.error.message):this.props.children;}
  }
  const evaluate=(expr,state,scope)=>api.evaluate(expr,state,scope);
  const propsFor=(plan,state,scope)=>Object.fromEntries(Object.entries(plan.attrs).map(([name,expr])=>
    [name,evaluate(expr,state,scope)]));
  function styleFor(plan,state,scope) {
    // Values remain CSS strings, matching the Rust/direct renderer's unit rules.
    return Object.fromEntries(Object.entries(plan.styles).map(([key,expr])=>{
      const value=evaluate(expr,state,scope);return [key,value==null?'':String(value)];
    }));
  }
  function useEvents(plan,state,scope,properties) {
    const owner=useRef({scope,active:true}), element=useRef(null), mounted=useRef(false);
    owner.current.scope=scope;
    const kind=Object.hasOwn(properties,'checked')?'checked':Object.hasOwn(properties,'value')?'value':null;
    const serverValue=kind?properties[kind]:undefined;
    const [draft,setDraft]=useState(serverValue);
    useLayoutEffect(()=>{
      if(!mounted.current) {
        mounted.current=true;
        if(kind&&element.current===document.activeElement)setDraft(element.current[kind]);
      } else if(kind&&api.pending()<=1)setDraft(serverValue);
    },[serverValue,kind]);
    useLayoutEffect(()=>()=>api.dispose(owner.current),[]);
    const events={};
    function invoke(prop,args) {
      const event=args.find(value=>value&&typeof value.preventDefault==='function');
      const target=event?.target, currentTarget=event?.currentTarget||element.current;
      const value=event?(target?.type==='checkbox'?target.checked:target?.value):args[0];
      if(kind&&['onChange','onInput','onValueChange','onCheckedChange'].includes(prop))setDraft(value);
      const spec=plan.events[prop];
      if(!spec)return;
      const eventName=event?.type||prop;
      const projected=event?(event.type==='submit'?null:{value,checked:target?.checked,key:event.key}):{value,checked:typeof value==='boolean'?value:undefined,args};
      api.fireEvent(owner.current,eventName,spec,currentTarget,event,projected);
    }
    for(const prop of Object.keys(plan.events))events[prop]=(...args)=>invoke(prop,args);
    // React's controlled fields need synchronous local input echo while Rust
    // executes the queued event. This is transient input UI, not app state.
    if(kind&&!events.onChange)events.onChange=(...args)=>invoke('onChange',args);
    return {events,element,draft,kind};
  }
  const childNode=(plan,state,scope,active,key)=>plan.kind==='text'?
    api.text(evaluate(plan.value,state,scope)):h(PlanNode,{key,plan,state,scope,active});
  function mergeProps(properties,injected) {
    const merged={...injected,...properties};
    for(const key of Object.keys(injected)) {
      if(/^on[A-Z]/.test(key)&&properties[key]&&injected[key])
        merged[key]=(...args)=>{properties[key](...args);injected[key](...args);};
      else if(key==='style')merged.style={...injected.style,...properties.style};
      else if(key==='className')merged.className=[injected.className,properties.className].filter(Boolean).join(' ');
    }
    return merged;
  }
  function nativeElement(plan,state,scope,active,properties,injected) {
    for(const key of ['href','src','action','poster','xlinkHref'])
      if(Object.hasOwn(properties,key)&&!api.safeURL(properties[key]))delete properties[key];
    properties.style=styleFor(plan,state,scope);
    const children=plan.tag==='textarea'&&Object.hasOwn(plan.attrs,'value')?[]:
      plan.children.map((child,index)=>childNode(child,state,scope,active,index));
    return h(plan.tag,mergeProps(properties,injected),...children);
  }
  function Host({plan,state,scope,active,injected}) {
    const properties=propsFor(plan,state,scope);
    const {events,element,draft,kind}=useEvents(plan,state,scope,properties);
    if(kind)properties[kind]=draft??(kind==='checked'?false:'');
    const composedRef=useComposedRefs(element,injected.ref);
    Object.assign(properties,events);
    return nativeElement(plan,state,scope,active,properties,{...injected,ref:composedRef});
  }
  function Imported({plan,state,scope,active,injected}) {
    const properties=propsFor(plan,state,scope);
    const {events,element,draft,kind}=useEvents(plan,state,scope,properties);
    if(kind)properties[kind]=draft;
    properties.style=styleFor(plan,state,scope);Object.assign(properties,events);
    properties.ref=useComposedRefs(element,injected.ref);
    let component=registry[plan.library];
    for(const segment of plan.export_name.split('.'))component=component?.[segment];
    if(!component)throw new Error(`Unregistered React component ${plan.library}:${plan.export_name}`);
    return h(component,mergeProps(properties,injected),...plan.children.map((child,index)=>
      childNode(child,state,scope,active,index)));
  }
  const PlanNode=memo(function PlanNode({plan,state,scope,active,...injected}) {
    stats.visited++;
    switch(plan.kind) {
      case 'text':return api.text(evaluate(plan.value,state,scope));
      case 'element':return Object.keys(plan.events).length||Object.hasOwn(plan.attrs,'value')||Object.hasOwn(plan.attrs,'checked')?
        h(Host,{plan,state,scope,active,injected}):nativeElement(plan,state,scope,active,propsFor(plan,state,scope),injected);
      case 'component':return active?h(Imported,{plan,state,scope,active,injected}):
        plan.fallback?h(PlanNode,{plan:plan.fallback,state,scope,active}):null;
      case 'fragment':return plan.children.map((child,key)=>h(PlanNode,{key,plan:child,state,scope,active}));
      case 'when':return h(PlanNode,{plan:api.truth(evaluate(plan.condition,state,scope))?plan.yes:plan.no,state,scope,active});
      case 'each': {
        const items=evaluate(plan.items,state,scope),seen=new Set();
        return (Array.isArray(items)?items:[]).map((item,index)=>{
          const local={...scope,[plan.name]:item,[plan.index]:index};
          const key=plan.body.key?api.text(evaluate(plan.body.key,state,local)):index;
          if(seen.has(key))throw new Error('Duplicate sibling key: '+key);seen.add(key);
          return h(PlanNode,{key,plan:plan.body,state,scope:local,active});
        });
      }
      default:throw new Error('Unknown native React node');
    }
  },(a,b)=>{
    const extra=Object.keys(a).filter(key=>!['plan','state','scope','active'].includes(key));
    const same=Object.keys(a).length===Object.keys(b).length&&extra.every(key=>Object.is(a[key],b[key]))&&a.plan===b.plan&&a.active===b.active&&a.plan._s.every(key=>Object.is(a.state[key],b.state[key]))&&
      a.plan._l.every(key=>Object.is(a.scope[key],b.scope[key]));
    if(same)stats.skipped++;return same;
  });
  function Page({payload,ticket}) {
    const [active,setActive]=useState(!hasComponents(payload.tree));
    useLayoutEffect(()=>{
      if(!active){setActive(true);return;}
      stats.commits++;
      if(!initialized){initialized=true;resolveFirst?.();}
      // React may coalesce updates. Every earlier caller is covered by this commit.
      for(const [id,waiter]of pendingCommit)if(id<=ticket){waiter.resolve();pendingCommit.delete(id);}
    });
    const dispatch=(name,args=[])=>api.fireEvent({scope:emptyScope,active:true},name,
      {name,args:args.map(value=>({op:'literal',value}))},null,null,{args,value:args[0]});
    return h(NanoStateContext.Provider,{value:payload.state},
      h(NanoEventContext.Provider,{value:dispatch},
        h(ErrorBoundary,{key:payload.title},h(PlanNode,{plan:payload.tree,state:payload.state,scope:emptyScope,active}))));
  }
  async function render(payload,hydrate) {
    current=payload;
    if(hasComponents(payload.tree))registry=await loadRegistry();
    if(reactRoot&&!initialized)await firstCommit;
    // Re-read the newest Rust snapshot after asynchronous chunk loading/hydration.
    payload=current;
    const ticket=++sequence;
    const committed=new Promise((resolve,reject)=>pendingCommit.set(ticket,{resolve,reject}));
    const node=h(Page,{payload,ticket});
    if(!reactRoot) {
      firstCommit=new Promise(resolve=>resolveFirst=resolve);
      const options={onRecoverableError:error=>{stats.recoverableErrors.push(error.message);api.showError(error.message);},onUncaughtError:fail};
      if(hydrate&&api.root.hasChildNodes()) {stats.hydration='hydrateRoot';reactRoot=hydrateRoot(api.root,node,options);}
      else {stats.hydration='createRoot';reactRoot=createRoot(api.root,options);reactRoot.render(node);}
    } else reactRoot.render(node);
    return committed;
  }
  return {render,stats:()=>stats};
}

window.__NANO_REACT__={React,createRoot,hydrateRoot,NanoStateContext,NanoEventContext,useNanoState,useNanoEvents};
window.__NANO_START__(createRenderer);
