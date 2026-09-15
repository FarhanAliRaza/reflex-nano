import {build} from 'vite';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {registry} from './components-registry.js';
const root=path.resolve(import.meta.dirname,'..');
const out=path.join(root,'crates/nano-core/frontend-dist');
await build({configFile:false,root,base:'./',logLevel:'warn',build:{outDir:out,emptyOutDir:true,
  target:'es2022',minify:true,cssCodeSplit:true,license:true,
  rollupOptions:{input:path.join(root,'frontend/react.jsx'),
    output:{entryFileNames:'react.js',chunkFileNames:'[name]-[hash].js',assetFileNames:'[name]-[hash][extname]'}}}});
const names={};
for(const [library,exports]of Object.entries(registry)) {
  names[library]=[];
  for(const [name,value]of Object.entries(exports)) {
    if(!/^[A-Z]/.test(name))continue;
    if(typeof value==='function'||value?.$$typeof)names[library].push(name);
    else if(value&&typeof value==='object')for(const [member,component]of Object.entries(value))
      if(typeof component==='function'||component?.$$typeof)names[library].push(`${name}.${member}`);
  }
  names[library].sort();
}
fs.writeFileSync(path.join(out,'registry.json'),JSON.stringify(names,null,2));
const files=['frontend/react.jsx','frontend/components.js','frontend/components-registry.js','frontend/custom.js',
  'frontend/build.mjs','frontend/package.json','package.json','crates/nano-core/src/client.js','package-lock.json'];
fs.writeFileSync(path.join(out,'sources.json'),JSON.stringify(Object.fromEntries(files.map(name=>[
  name,crypto.createHash('sha256').update(fs.readFileSync(path.join(root,name))).digest('hex')
])),null,2));
console.log('React frontend built:',fs.readdirSync(out).join(', '));
