// A minimal stand-in for the mockup tool's runtime, enough to render
// docs/design/mockup-D.dc.html in a plain browser: it runs the component's
// renderVals(), expands <sc-for> / <sc-if>, substitutes {{expressions}} in
// attributes and text, and moves <helmet> children into <head>.
// Used only by render_mockup.ps1 (the pixel comparison, DESIGNPASS.md).
class DCLogic {}
window.DCLogic = DCLogic;

function dcEval(expr, scope) {
  const keys = Object.keys(scope);
  return new Function(...keys, 'return (' + expr + ');')(...keys.map((k) => scope[k]));
}

function dcSubst(text, scope) {
  return text.replace(/\{\{([\s\S]+?)\}\}/g, (_, e) => {
    const v = dcEval(e, scope);
    return v === undefined || v === null ? '' : String(v);
  });
}

function dcExpand(node, scope) {
  for (const child of [...node.childNodes]) {
    if (child.nodeType === Node.TEXT_NODE) {
      if (child.nodeValue.includes('{{')) child.nodeValue = dcSubst(child.nodeValue, scope);
      continue;
    }
    if (child.nodeType !== Node.ELEMENT_NODE) continue;
    const tag = child.tagName.toLowerCase();
    if (tag === 'sc-for') {
      const list = dcEval(child.getAttribute('list').replace(/^\{\{|\}\}$/g, ''), scope) || [];
      const as = child.getAttribute('as');
      for (const item of list) {
        const holder = document.createElement('div');
        for (const c of child.childNodes) holder.appendChild(c.cloneNode(true));
        dcExpand(holder, { ...scope, [as]: item });
        for (const c of [...holder.childNodes]) node.insertBefore(c, child);
      }
      child.remove();
    } else if (tag === 'sc-if') {
      const v = dcEval(child.getAttribute('value').replace(/^\{\{|\}\}$/g, ''), scope);
      if (v) {
        dcExpand(child, scope);
        for (const c of [...child.childNodes]) node.insertBefore(c, child);
      }
      child.remove();
    } else {
      for (const a of [...child.attributes]) {
        if (a.name.startsWith('hint-')) child.removeAttribute(a.name);
        else if (a.value.includes('{{')) child.setAttribute(a.name, dcSubst(a.value, scope));
      }
      dcExpand(child, scope);
    }
  }
}

document.addEventListener('DOMContentLoaded', () => {
  const src = document.querySelector('script[type="text/x-dc"]').textContent;
  const Component = new Function('DCLogic', src + '\nreturn Component;')(DCLogic);
  const vals = new Component().renderVals();
  const root = document.querySelector('x-dc');
  const helmet = root.querySelector('helmet');
  if (helmet) {
    for (const n of [...helmet.childNodes]) document.head.appendChild(n);
    helmet.remove();
  }
  dcExpand(root, vals);
  document.documentElement.setAttribute('data-dc-ready', '1');
  // ?measure: after the fonts load, write every element's box (and every
  // text run's ink-free line box) into a <pre id="measure"> as JSON lines,
  // for `chrome --dump-dom`.
  if (location.search.includes('measure')) {
    document.fonts.ready.then(() => {
      const out = [];
      const all = root.querySelectorAll('*');
      all.forEach((el, i) => {
        const r = el.getBoundingClientRect();
        const own = [...el.childNodes].filter((n) => n.nodeType === Node.TEXT_NODE).map((n) => n.nodeValue.trim()).join('');
        out.push({ i, tag: el.tagName.toLowerCase(), label: el.getAttribute('aria-label') || '', text: own,
          x: +r.x.toFixed(2), y: +r.y.toFixed(2), w: +r.width.toFixed(2), h: +r.height.toFixed(2) });
        if (own) {
          for (const n of el.childNodes) {
            if (n.nodeType !== Node.TEXT_NODE || !n.nodeValue.trim()) continue;
            const rg = document.createRange();
            rg.selectNodeContents(n);
            const t = rg.getBoundingClientRect();
            out.push({ i, tag: '#text', text: n.nodeValue.trim(), x: +t.x.toFixed(2), y: +t.y.toFixed(2), w: +t.width.toFixed(2), h: +t.height.toFixed(2) });
          }
        }
      });
      const pre = document.createElement('pre');
      pre.id = 'measure';
      pre.textContent = out.map((o) => JSON.stringify(o)).join('\n');
      document.body.appendChild(pre);
    });
  }
});
