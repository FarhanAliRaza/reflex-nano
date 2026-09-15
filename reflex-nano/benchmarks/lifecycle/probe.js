/* Runs before app JavaScript. Both frameworks use identical state/DOM checks. */
({rows, kind, expectedCount}) => {
  const m = window.__lifecycle = {rows, kind, expectedCount, state: {}, marks: {},
    websocket_in_bytes: 0, websocket_out_bytes: 0, incoming_frames: 0, errors: []};
  const now = () => performance.now();
  const mark = name => {if (m.marks[name] === undefined) m.marks[name] = now();};
  let validatedState = false;
  const validItems = items => Array.isArray(items) && items.length === rows &&
    items.every((v, i) => v.id === i && v.label === `Item ${i}` && v.done === false);
  const text = id => document.getElementById(id)?.textContent;
  function check() {
    if (m.marks.ready_ms !== undefined || !validatedState) return;
    if (kind === 'reflex' ? text('hydrated') !== 'true' : ((kind !== 'nano_before' && !window.__NANO__?.ready) || !window.__NANO__?.connection().connected)) return;
    if (text('count') !== String(expectedCount) || text('doubled') !== String(expectedCount * 2) ||
        text('remaining') !== String(rows) || text('name') !== 'Nano' || text('progress') !== '0') return;
    const nodes = document.querySelectorAll('#items > .item');
    if (nodes.length !== rows) return;
    for (let i = 0; i < rows; i++) {
      const node = nodes[i];
      if (node.querySelector('.item-id')?.textContent !== String(i) ||
          node.querySelector('.item-label')?.textContent !== `Item ${i}` ||
          node.querySelector('.item-done')?.textContent !== 'false' ||
          !node.querySelector('button')) return;
    }
    mark('ready_ms'); observer.disconnect();
    requestAnimationFrame(() => requestAnimationFrame(() => mark('paint_ready_ms')));
  }
  const observer = new MutationObserver(check);
  observer.observe(document, {subtree: true, childList: true, characterData: true, attributes: true});
  window.addEventListener('nano:connected', check);
  window.addEventListener('nano:hydrated', check);
  window.addEventListener('nano:update', check);
  window.addEventListener('error', e => m.errors.push(e.message));
  window.addEventListener('unhandledrejection', e => m.errors.push(String(e.reason)));
  new PerformanceObserver(list => {
    for (const e of list.getEntries()) m.marks[e.name.replaceAll('-', '_') + '_ms'] = e.startTime;
  }).observe({type: 'paint', buffered: true});
  const Original = window.WebSocket;
  window.WebSocket = class extends Original {
    constructor(...args) {
      super(...args);
      this.addEventListener('open', () => mark('websocket_open_ms'));
      this.addEventListener('message', e => {
        if (typeof e.data !== 'string') return;
        m.websocket_in_bytes += new TextEncoder().encode(e.data).length;
        m.incoming_frames++; mark('first_frame_ms');
        try {
          if (kind !== 'reflex') {
            const packet = JSON.parse(e.data);
            Object.assign(m.state, packet.type === 'snapshot' ? packet.state : packet.delta || {});
          } else if (e.data.startsWith('42/_event,')) {
            const packet = JSON.parse(e.data.slice('42/_event,'.length))[1];
            for (const [name, delta] of Object.entries(packet.delta || {})) {
              if (name.endsWith('runtime_state')) for (const [key, val] of Object.entries(delta))
                m.state[key.replace(/_rx_state_$/, '')] = val;
            }
          }
          if (!validatedState && m.state.count === expectedCount && m.state.doubled === expectedCount * 2 &&
              m.state.name === 'Nano' && m.state.progress === 0 && m.state.remaining === rows && validItems(m.state.items)) {
            validatedState = true; mark('full_state_ms');
          }
        } catch (error) {m.errors.push(error.message);}
        queueMicrotask(check); setTimeout(check, 0);
      });
    }
    send(data) {
      m.websocket_out_bytes += typeof data === 'string' ? new TextEncoder().encode(data).length : data.byteLength || 0;
      return super.send(data);
    }
  };
  m.event = (selector, field, expected, computedField, computedExpected) => new Promise((resolve, reject) => {
    const start = now();
    const timer = setTimeout(() => {o.disconnect();reject(new Error('Event verification timed out'));}, 30000);
    const o = new MutationObserver(() => {
      if (text(field) === String(expected) && text(computedField) === String(computedExpected)) {
        o.disconnect(); clearTimeout(timer); resolve(now() - start);
      }
    });
    o.observe(document, {subtree: true, childList: true, characterData: true, attributes: true});
    document.querySelector(selector).click();
  });
}
