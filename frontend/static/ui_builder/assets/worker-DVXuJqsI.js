var We={exports:{}},st;function qt(){return st||(st=1,function(_){(p=>{var O=Object.defineProperty,I=Object.getOwnPropertyDescriptor,N=Object.getOwnPropertyNames,G=Object.prototype.hasOwnProperty,X=(e,n)=>{for(var r in n)O(e,r,{get:n[r],enumerable:!0})},te=(e,n,r,c)=>{if(n&&typeof n=="object"||typeof n=="function")for(let g of N(n))!G.call(e,g)&&g!==r&&O(e,g,{get:()=>n[g],enumerable:!(c=I(n,g))||c.enumerable});return e},J=e=>te(O({},"__esModule",{value:!0}),e),ie=(e,n,r)=>new Promise((c,g)=>{var w=u=>{try{S(r.next(u))}catch(F){g(F)}},h=u=>{try{S(r.throw(u))}catch(F){g(F)}},S=u=>u.done?c(u.value):Promise.resolve(u.value).then(w,h);S((r=r.apply(e,n)).next())}),Q={};X(Q,{analyzeMetafile:()=>At,analyzeMetafileSync:()=>Ft,build:()=>Ct,buildSync:()=>Ut,context:()=>Dt,default:()=>Wt,formatMessages:()=>Rt,formatMessagesSync:()=>Nt,initialize:()=>Vt,stop:()=>Lt,transform:()=>It,transformSync:()=>Mt,version:()=>Pt}),p.exports=J(Q);function me(e){let n=c=>{if(c===null)r.write8(0);else if(typeof c=="boolean")r.write8(1),r.write8(+c);else if(typeof c=="number")r.write8(2),r.write32(c|0);else if(typeof c=="string")r.write8(3),r.write(K(c));else if(c instanceof Uint8Array)r.write8(4),r.write(c);else if(c instanceof Array){r.write8(5),r.write32(c.length);for(let g of c)n(g)}else{let g=Object.keys(c);r.write8(6),r.write32(g.length);for(let w of g)r.write(K(w)),n(c[w])}},r=new ce;return r.write32(0),r.write32(e.id<<1|+!e.isRequest),n(e.value),De(r.buf,r.len-4,0),r.buf.subarray(0,r.len)}function ae(e){let n=()=>{switch(r.read8()){case 0:return null;case 1:return!!r.read8();case 2:return r.read32();case 3:return pe(r.read());case 4:return r.read();case 5:{let h=r.read32(),S=[];for(let u=0;u<h;u++)S.push(n());return S}case 6:{let h=r.read32(),S={};for(let u=0;u<h;u++)S[pe(r.read())]=n();return S}default:throw new Error("Invalid packet")}},r=new ce(e),c=r.read32(),g=(c&1)===0;c>>>=1;let w=n();if(r.ptr!==e.length)throw new Error("Invalid packet");return{id:c,isRequest:g,value:w}}var ce=class{constructor(e=new Uint8Array(1024)){this.buf=e,this.len=0,this.ptr=0}_write(e){if(this.len+e>this.buf.length){let n=new Uint8Array((this.len+e)*2);n.set(this.buf),this.buf=n}return this.len+=e,this.len-e}write8(e){let n=this._write(1);this.buf[n]=e}write32(e){let n=this._write(4);De(this.buf,e,n)}write(e){let n=this._write(4+e.length);De(this.buf,e.length,n),this.buf.set(e,n+4)}_read(e){if(this.ptr+e>this.buf.length)throw new Error("Invalid packet");return this.ptr+=e,this.ptr-e}read8(){return this.buf[this._read(1)]}read32(){return $e(this.buf,this._read(4))}read(){let e=this.read32(),n=new Uint8Array(e),r=this._read(n.length);return n.set(this.buf.subarray(r,r+e)),n}},K,pe,_e;if(typeof TextEncoder<"u"&&typeof TextDecoder<"u"){let e=new TextEncoder,n=new TextDecoder;K=r=>e.encode(r),pe=r=>n.decode(r),_e='new TextEncoder().encode("")'}else if(typeof Buffer<"u")K=e=>Buffer.from(e),pe=e=>{let{buffer:n,byteOffset:r,byteLength:c}=e;return Buffer.from(n,r,c).toString()},_e='Buffer.from("")';else throw new Error("No UTF-8 codec found");if(!(K("")instanceof Uint8Array))throw new Error(`Invariant violation: "${_e} instanceof Uint8Array" is incorrectly false

This indicates that your JavaScript environment is broken. You cannot use
esbuild in this environment because esbuild relies on this invariant. This
is not a problem with esbuild. You need to fix your environment instead.
`);function $e(e,n){return e[n++]|e[n++]<<8|e[n++]<<16|e[n++]<<24}function De(e,n,r){e[r++]=n,e[r++]=n>>8,e[r++]=n>>16,e[r++]=n>>24}var H=JSON.stringify,ye="warning",ve="silent";function Te(e){if(oe(e,"target"),e.indexOf(",")>=0)throw new Error(`Invalid target: ${e}`);return e}var Ie=()=>null,Z=e=>typeof e=="boolean"?null:"a boolean",C=e=>typeof e=="string"?null:"a string",Re=e=>e instanceof RegExp?null:"a RegExp object",Ee=e=>typeof e=="number"&&e===(e|0)?null:"an integer",gt=e=>typeof e=="number"&&e===(e|0)&&e>=0&&e<=65535?null:"a valid port number",He=e=>typeof e=="function"?null:"a function",re=e=>Array.isArray(e)?null:"an array",fe=e=>typeof e=="object"&&e!==null&&!Array.isArray(e)?null:"an object",yt=e=>typeof e=="object"&&e!==null?null:"an array or an object",wt=e=>e instanceof WebAssembly.Module?null:"a WebAssembly.Module",Xe=e=>typeof e=="object"&&!Array.isArray(e)?null:"an object or null",Qe=e=>typeof e=="string"||typeof e=="boolean"?null:"a string or a boolean",vt=e=>typeof e=="string"||typeof e=="object"&&e!==null&&!Array.isArray(e)?null:"a string or an object",bt=e=>typeof e=="string"||Array.isArray(e)?null:"a string or an array",Ke=e=>typeof e=="string"||e instanceof Uint8Array?null:"a string or a Uint8Array",xt=e=>typeof e=="string"||e instanceof URL?null:"a string or a URL";function i(e,n,r,c){let g=e[r];if(n[r+""]=!0,g===void 0)return;let w=c(g);if(w!==null)throw new Error(`${H(r)} must be ${w}`);return g}function le(e,n,r){for(let c in e)if(!(c in n))throw new Error(`Invalid option ${r}: ${H(c)}`)}function _t(e){let n=Object.create(null),r=i(e,n,"wasmURL",xt),c=i(e,n,"wasmModule",wt),g=i(e,n,"worker",Z);return le(e,n,"in initialize() call"),{wasmURL:r,wasmModule:c,worker:g}}function Ze(e){let n;if(e!==void 0){n=Object.create(null);for(let r in e){let c=e[r];if(typeof c=="string"||c===!1)n[r]=c;else throw new Error(`Expected ${H(r)} in mangle cache to map to either a string or false`)}}return n}function Ae(e,n,r,c,g){let w=i(n,r,"color",Z),h=i(n,r,"logLevel",C),S=i(n,r,"logLimit",Ee);w!==void 0?e.push(`--color=${w}`):c&&e.push("--color=true"),e.push(`--log-level=${h||g}`),e.push(`--log-limit=${S||0}`)}function oe(e,n,r){if(typeof e!="string")throw new Error(`Expected value for ${n}${r!==void 0?" "+H(r):""} to be a string, got ${typeof e} instead`);return e}function et(e,n,r){let c=i(n,r,"legalComments",C),g=i(n,r,"sourceRoot",C),w=i(n,r,"sourcesContent",Z),h=i(n,r,"target",bt),S=i(n,r,"format",C),u=i(n,r,"globalName",C),F=i(n,r,"mangleProps",Re),L=i(n,r,"reserveProps",Re),D=i(n,r,"mangleQuoted",Z),q=i(n,r,"minify",Z),U=i(n,r,"minifySyntax",Z),B=i(n,r,"minifyWhitespace",Z),W=i(n,r,"minifyIdentifiers",Z),P=i(n,r,"lineLimit",Ee),ne=i(n,r,"drop",re),T=i(n,r,"dropLabels",re),$=i(n,r,"charset",C),y=i(n,r,"treeShaking",Z),f=i(n,r,"ignoreAnnotations",Z),s=i(n,r,"jsx",C),o=i(n,r,"jsxFactory",C),d=i(n,r,"jsxFragment",C),x=i(n,r,"jsxImportSource",C),E=i(n,r,"jsxDev",Z),a=i(n,r,"jsxSideEffects",Z),m=i(n,r,"define",fe),t=i(n,r,"logOverride",fe),l=i(n,r,"supported",fe),v=i(n,r,"pure",re),b=i(n,r,"keepNames",Z),j=i(n,r,"platform",C),M=i(n,r,"tsconfigRaw",vt);if(c&&e.push(`--legal-comments=${c}`),g!==void 0&&e.push(`--source-root=${g}`),w!==void 0&&e.push(`--sources-content=${w}`),h&&(Array.isArray(h)?e.push(`--target=${Array.from(h).map(Te).join(",")}`):e.push(`--target=${Te(h)}`)),S&&e.push(`--format=${S}`),u&&e.push(`--global-name=${u}`),j&&e.push(`--platform=${j}`),M&&e.push(`--tsconfig-raw=${typeof M=="string"?M:JSON.stringify(M)}`),q&&e.push("--minify"),U&&e.push("--minify-syntax"),B&&e.push("--minify-whitespace"),W&&e.push("--minify-identifiers"),P&&e.push(`--line-limit=${P}`),$&&e.push(`--charset=${$}`),y!==void 0&&e.push(`--tree-shaking=${y}`),f&&e.push("--ignore-annotations"),ne)for(let A of ne)e.push(`--drop:${oe(A,"drop")}`);if(T&&e.push(`--drop-labels=${Array.from(T).map(A=>oe(A,"dropLabels")).join(",")}`),F&&e.push(`--mangle-props=${Ne(F)}`),L&&e.push(`--reserve-props=${Ne(L)}`),D!==void 0&&e.push(`--mangle-quoted=${D}`),s&&e.push(`--jsx=${s}`),o&&e.push(`--jsx-factory=${o}`),d&&e.push(`--jsx-fragment=${d}`),x&&e.push(`--jsx-import-source=${x}`),E&&e.push("--jsx-dev"),a&&e.push("--jsx-side-effects"),m)for(let A in m){if(A.indexOf("=")>=0)throw new Error(`Invalid define: ${A}`);e.push(`--define:${A}=${oe(m[A],"define",A)}`)}if(t)for(let A in t){if(A.indexOf("=")>=0)throw new Error(`Invalid log override: ${A}`);e.push(`--log-override:${A}=${oe(t[A],"log override",A)}`)}if(l)for(let A in l){if(A.indexOf("=")>=0)throw new Error(`Invalid supported: ${A}`);const R=l[A];if(typeof R!="boolean")throw new Error(`Expected value for supported ${H(A)} to be a boolean, got ${typeof R} instead`);e.push(`--supported:${A}=${R}`)}if(v)for(let A of v)e.push(`--pure:${oe(A,"pure")}`);b&&e.push("--keep-names")}function Et(e,n,r,c,g){var w;let h=[],S=[],u=Object.create(null),F=null,L=null;Ae(h,n,u,r,c),et(h,n,u);let D=i(n,u,"sourcemap",Qe),q=i(n,u,"bundle",Z),U=i(n,u,"splitting",Z),B=i(n,u,"preserveSymlinks",Z),W=i(n,u,"metafile",Z),P=i(n,u,"outfile",C),ne=i(n,u,"outdir",C),T=i(n,u,"outbase",C),$=i(n,u,"tsconfig",C),y=i(n,u,"resolveExtensions",re),f=i(n,u,"nodePaths",re),s=i(n,u,"mainFields",re),o=i(n,u,"conditions",re),d=i(n,u,"external",re),x=i(n,u,"packages",C),E=i(n,u,"alias",fe),a=i(n,u,"loader",fe),m=i(n,u,"outExtension",fe),t=i(n,u,"publicPath",C),l=i(n,u,"entryNames",C),v=i(n,u,"chunkNames",C),b=i(n,u,"assetNames",C),j=i(n,u,"inject",re),M=i(n,u,"banner",fe),A=i(n,u,"footer",fe),R=i(n,u,"entryPoints",yt),Y=i(n,u,"absWorkingDir",C),V=i(n,u,"stdin",fe),z=(w=i(n,u,"write",Z))!=null?w:g,de=i(n,u,"allowOverwrite",Z),se=i(n,u,"mangleCache",fe);if(u.plugins=!0,le(n,u,`in ${e}() call`),D&&h.push(`--sourcemap${D===!0?"":`=${D}`}`),q&&h.push("--bundle"),de&&h.push("--allow-overwrite"),U&&h.push("--splitting"),B&&h.push("--preserve-symlinks"),W&&h.push("--metafile"),P&&h.push(`--outfile=${P}`),ne&&h.push(`--outdir=${ne}`),T&&h.push(`--outbase=${T}`),$&&h.push(`--tsconfig=${$}`),x&&h.push(`--packages=${x}`),y){let k=[];for(let ee of y){if(oe(ee,"resolve extension"),ee.indexOf(",")>=0)throw new Error(`Invalid resolve extension: ${ee}`);k.push(ee)}h.push(`--resolve-extensions=${k.join(",")}`)}if(t&&h.push(`--public-path=${t}`),l&&h.push(`--entry-names=${l}`),v&&h.push(`--chunk-names=${v}`),b&&h.push(`--asset-names=${b}`),s){let k=[];for(let ee of s){if(oe(ee,"main field"),ee.indexOf(",")>=0)throw new Error(`Invalid main field: ${ee}`);k.push(ee)}h.push(`--main-fields=${k.join(",")}`)}if(o){let k=[];for(let ee of o){if(oe(ee,"condition"),ee.indexOf(",")>=0)throw new Error(`Invalid condition: ${ee}`);k.push(ee)}h.push(`--conditions=${k.join(",")}`)}if(d)for(let k of d)h.push(`--external:${oe(k,"external")}`);if(E)for(let k in E){if(k.indexOf("=")>=0)throw new Error(`Invalid package name in alias: ${k}`);h.push(`--alias:${k}=${oe(E[k],"alias",k)}`)}if(M)for(let k in M){if(k.indexOf("=")>=0)throw new Error(`Invalid banner file type: ${k}`);h.push(`--banner:${k}=${oe(M[k],"banner",k)}`)}if(A)for(let k in A){if(k.indexOf("=")>=0)throw new Error(`Invalid footer file type: ${k}`);h.push(`--footer:${k}=${oe(A[k],"footer",k)}`)}if(j)for(let k of j)h.push(`--inject:${oe(k,"inject")}`);if(a)for(let k in a){if(k.indexOf("=")>=0)throw new Error(`Invalid loader extension: ${k}`);h.push(`--loader:${k}=${oe(a[k],"loader",k)}`)}if(m)for(let k in m){if(k.indexOf("=")>=0)throw new Error(`Invalid out extension: ${k}`);h.push(`--out-extension:${k}=${oe(m[k],"out extension",k)}`)}if(R)if(Array.isArray(R))for(let k=0,ee=R.length;k<ee;k++){let he=R[k];if(typeof he=="object"&&he!==null){let ge=Object.create(null),ue=i(he,ge,"in",C),Pe=i(he,ge,"out",C);if(le(he,ge,"in entry point at index "+k),ue===void 0)throw new Error('Missing property "in" for entry point at index '+k);if(Pe===void 0)throw new Error('Missing property "out" for entry point at index '+k);S.push([Pe,ue])}else S.push(["",oe(he,"entry point at index "+k)])}else for(let k in R)S.push([k,oe(R[k],"entry point",k)]);if(V){let k=Object.create(null),ee=i(V,k,"contents",Ke),he=i(V,k,"resolveDir",C),ge=i(V,k,"sourcefile",C),ue=i(V,k,"loader",C);le(V,k,'in "stdin" object'),ge&&h.push(`--sourcefile=${ge}`),ue&&h.push(`--loader=${ue}`),he&&(L=he),typeof ee=="string"?F=K(ee):ee instanceof Uint8Array&&(F=ee)}let Se=[];if(f)for(let k of f)k+="",Se.push(k);return{entries:S,flags:h,write:z,stdinContents:F,stdinResolveDir:L,absWorkingDir:Y,nodePaths:Se,mangleCache:Ze(se)}}function kt(e,n,r,c){let g=[],w=Object.create(null);Ae(g,n,w,r,c),et(g,n,w);let h=i(n,w,"sourcemap",Qe),S=i(n,w,"sourcefile",C),u=i(n,w,"loader",C),F=i(n,w,"banner",C),L=i(n,w,"footer",C),D=i(n,w,"mangleCache",fe);return le(n,w,`in ${e}() call`),h&&g.push(`--sourcemap=${h===!0?"external":h}`),S&&g.push(`--sourcefile=${S}`),u&&g.push(`--loader=${u}`),F&&g.push(`--banner=${F}`),L&&g.push(`--footer=${L}`),{flags:g,mangleCache:Ze(D)}}function St(e){const n={},r={didClose:!1,reason:""};let c={},g=0,w=0,h=new Uint8Array(16*1024),S=0,u=$=>{let y=S+$.length;if(y>h.length){let s=new Uint8Array(y*2);s.set(h),h=s}h.set($,S),S+=$.length;let f=0;for(;f+4<=S;){let s=$e(h,f);if(f+4+s>S)break;f+=4,B(h.subarray(f,f+s)),f+=s}f>0&&(h.copyWithin(0,f,S),S-=f)},F=$=>{r.didClose=!0,$&&(r.reason=": "+($.message||$));const y="The service was stopped"+r.reason;for(let f in c)c[f](y,null);c={}},L=($,y,f)=>{if(r.didClose)return f("The service is no longer running"+r.reason,null);let s=g++;c[s]=(o,d)=>{try{f(o,d)}finally{$&&$.unref()}},$&&$.ref(),e.writeToStdin(me({id:s,isRequest:!0,value:y}))},D=($,y)=>{if(r.didClose)throw new Error("The service is no longer running"+r.reason);e.writeToStdin(me({id:$,isRequest:!1,value:y}))},q=($,y)=>ie(this,null,function*(){try{if(y.command==="ping"){D($,{});return}if(typeof y.key=="number"){const f=n[y.key];if(!f)return;const s=f[y.command];if(s){yield s($,y);return}}throw new Error("Invalid command: "+y.command)}catch(f){const s=[be(f,e,null,void 0,"")];try{D($,{errors:s})}catch{}}}),U=!0,B=$=>{if(U){U=!1;let f=String.fromCharCode(...$);if(f!=="0.25.2")throw new Error(`Cannot start service: Host version "0.25.2" does not match binary version ${H(f)}`);return}let y=ae($);if(y.isRequest)q(y.id,y.value);else{let f=c[y.id];delete c[y.id],y.value.error?f(y.value.error,{}):f(null,y.value)}};return{readFromStdout:u,afterClose:F,service:{buildOrContext:({callName:$,refs:y,options:f,isTTY:s,defaultWD:o,callback:d})=>{let x=0;const E=w++,a={},m={ref(){++x===1&&y&&y.ref()},unref(){--x===0&&(delete n[E],y&&y.unref())}};n[E]=a,m.ref(),$t($,E,L,D,m,e,a,f,s,o,(t,l)=>{try{d(t,l)}finally{m.unref()}})},transform:({callName:$,refs:y,input:f,options:s,isTTY:o,fs:d,callback:x})=>{const E=tt();let a=m=>{try{if(typeof f!="string"&&!(f instanceof Uint8Array))throw new Error('The input to "transform" must be a string or a Uint8Array');let{flags:t,mangleCache:l}=kt($,s,o,ve),v={command:"transform",flags:t,inputFS:m!==null,input:m!==null?K(m):typeof f=="string"?K(f):f};l&&(v.mangleCache=l),L(y,v,(b,j)=>{if(b)return x(new Error(b),null);let M=ke(j.errors,E),A=ke(j.warnings,E),R=1,Y=()=>{if(--R===0){let V={warnings:A,code:j.code,map:j.map,mangleCache:void 0,legalComments:void 0};"legalComments"in j&&(V.legalComments=j==null?void 0:j.legalComments),j.mangleCache&&(V.mangleCache=j==null?void 0:j.mangleCache),x(null,V)}};if(M.length>0)return x(je("Transform failed",M,A),null);j.codeFS&&(R++,d.readFile(j.code,(V,z)=>{V!==null?x(V,null):(j.code=z,Y())})),j.mapFS&&(R++,d.readFile(j.map,(V,z)=>{V!==null?x(V,null):(j.map=z,Y())})),Y()})}catch(t){let l=[];try{Ae(l,s,{},o,ve)}catch{}const v=be(t,e,E,void 0,"");L(y,{command:"error",flags:l,error:v},()=>{v.detail=E.load(v.detail),x(je("Transform failed",[v],[]),null)})}};if((typeof f=="string"||f instanceof Uint8Array)&&f.length>1024*1024){let m=a;a=()=>d.writeFile(f,m)}a(null)},formatMessages:({callName:$,refs:y,messages:f,options:s,callback:o})=>{if(!s)throw new Error(`Missing second argument in ${$}() call`);let d={},x=i(s,d,"kind",C),E=i(s,d,"color",Z),a=i(s,d,"terminalWidth",Ee);if(le(s,d,`in ${$}() call`),x===void 0)throw new Error(`Missing "kind" in ${$}() call`);if(x!=="error"&&x!=="warning")throw new Error(`Expected "kind" to be "error" or "warning" in ${$}() call`);let m={command:"format-msgs",messages:we(f,"messages",null,"",a),isWarning:x==="warning"};E!==void 0&&(m.color=E),a!==void 0&&(m.terminalWidth=a),L(y,m,(t,l)=>{if(t)return o(new Error(t),null);o(null,l.messages)})},analyzeMetafile:({callName:$,refs:y,metafile:f,options:s,callback:o})=>{s===void 0&&(s={});let d={},x=i(s,d,"color",Z),E=i(s,d,"verbose",Z);le(s,d,`in ${$}() call`);let a={command:"analyze-metafile",metafile:f};x!==void 0&&(a.color=x),E!==void 0&&(a.verbose=E),L(y,a,(m,t)=>{if(m)return o(new Error(m),null);o(null,t.result)})}}}}function $t(e,n,r,c,g,w,h,S,u,F,L){const D=tt(),q=e==="context",U=(P,ne)=>{const T=[];try{Ae(T,S,{},u,ye)}catch{}const $=be(P,w,D,void 0,ne);r(g,{command:"error",flags:T,error:$},()=>{$.detail=D.load($.detail),L(je(q?"Context failed":"Build failed",[$],[]),null)})};let B;if(typeof S=="object"){const P=S.plugins;if(P!==void 0){if(!Array.isArray(P))return U(new Error('"plugins" must be an array'),"");B=P}}if(B&&B.length>0){if(w.isSync)return U(new Error("Cannot use plugins in synchronous API calls"),"");Tt(n,r,c,g,w,h,S,B,D).then(P=>{if(!P.ok)return U(P.error,P.pluginName);try{W(P.requestPlugins,P.runOnEndCallbacks,P.scheduleOnDisposeCallbacks)}catch(ne){U(ne,"")}},P=>U(P,""));return}try{W(null,(P,ne)=>ne([],[]),()=>{})}catch(P){U(P,"")}function W(P,ne,T){const $=w.hasFS,{entries:y,flags:f,write:s,stdinContents:o,stdinResolveDir:d,absWorkingDir:x,nodePaths:E,mangleCache:a}=Et(e,S,u,ye,$);if(s&&!w.hasFS)throw new Error('The "write" option is unavailable in this environment');const m={command:"build",key:n,entries:y,flags:f,write:s,stdinContents:o,stdinResolveDir:d,absWorkingDir:x||F,nodePaths:E,context:q};P&&(m.plugins=P),a&&(m.mangleCache=a);const t=(b,j)=>{const M={errors:ke(b.errors,D),warnings:ke(b.warnings,D),outputFiles:void 0,metafile:void 0,mangleCache:void 0},A=M.errors.slice(),R=M.warnings.slice();b.outputFiles&&(M.outputFiles=b.outputFiles.map(Ot)),b.metafile&&(M.metafile=JSON.parse(b.metafile)),b.mangleCache&&(M.mangleCache=b.mangleCache),b.writeToStdout!==void 0&&console.log(pe(b.writeToStdout).replace(/\n$/,"")),ne(M,(Y,V)=>{if(A.length>0||Y.length>0){const z=je("Build failed",A.concat(Y),R.concat(V));return j(z,null,Y,V)}j(null,M,Y,V)})};let l,v;q&&(h["on-end"]=(b,j)=>new Promise(M=>{t(j,(A,R,Y,V)=>{const z={errors:Y,warnings:V};v&&v(A,R),l=void 0,v=void 0,c(b,z),M()})})),r(g,m,(b,j)=>{if(b)return L(new Error(b),null);if(!q)return t(j,(R,Y)=>(T(),L(R,Y)));if(j.errors.length>0)return L(je("Context failed",j.errors,j.warnings),null);let M=!1;const A={rebuild:()=>(l||(l=new Promise((R,Y)=>{let V;v=(de,se)=>{V||(V=()=>de?Y(de):R(se))};const z=()=>{r(g,{command:"rebuild",key:n},(se,Se)=>{se?Y(new Error(se)):V?V():z()})};z()})),l),watch:(R={})=>new Promise((Y,V)=>{if(!w.hasFS)throw new Error('Cannot use the "watch" API in this environment');le(R,{},"in watch() call"),r(g,{command:"watch",key:n},se=>{se?V(new Error(se)):Y(void 0)})}),serve:(R={})=>new Promise((Y,V)=>{if(!w.hasFS)throw new Error('Cannot use the "serve" API in this environment');const z={},de=i(R,z,"port",gt),se=i(R,z,"host",C),Se=i(R,z,"servedir",C),k=i(R,z,"keyfile",C),ee=i(R,z,"certfile",C),he=i(R,z,"fallback",C),ge=i(R,z,"onRequest",He);le(R,z,"in serve() call");const ue={command:"serve",key:n,onRequest:!!ge};de!==void 0&&(ue.port=de),se!==void 0&&(ue.host=se),Se!==void 0&&(ue.servedir=Se),k!==void 0&&(ue.keyfile=k),ee!==void 0&&(ue.certfile=ee),he!==void 0&&(ue.fallback=he),r(g,ue,(Pe,zt)=>{if(Pe)return V(new Error(Pe));ge&&(h["serve-request"]=(Gt,Jt)=>{ge(Jt.args),c(Gt,{})}),Y(zt)})}),cancel:()=>new Promise(R=>{if(M)return R();r(g,{command:"cancel",key:n},()=>{R()})}),dispose:()=>new Promise(R=>{if(M)return R();M=!0,r(g,{command:"dispose",key:n},()=>{R(),T(),g.unref()})})};g.ref(),L(null,A)})}}var Tt=(e,n,r,c,g,w,h,S,u)=>ie(void 0,null,function*(){let F=[],L=[],D={},q={},U=[],B=0,W=0,P=[],ne=!1;S=[...S];for(let y of S){let f={};if(typeof y!="object")throw new Error(`Plugin at index ${W} must be an object`);const s=i(y,f,"name",C);if(typeof s!="string"||s==="")throw new Error(`Plugin at index ${W} is missing a name`);try{let o=i(y,f,"setup",He);if(typeof o!="function")throw new Error("Plugin is missing a setup function");le(y,f,`on plugin ${H(s)}`);let d={name:s,onStart:!1,onEnd:!1,onResolve:[],onLoad:[]};W++;let E=o({initialOptions:h,resolve:(a,m={})=>{if(!ne)throw new Error('Cannot call "resolve" before plugin setup has completed');if(typeof a!="string")throw new Error("The path to resolve must be a string");let t=Object.create(null),l=i(m,t,"pluginName",C),v=i(m,t,"importer",C),b=i(m,t,"namespace",C),j=i(m,t,"resolveDir",C),M=i(m,t,"kind",C),A=i(m,t,"pluginData",Ie),R=i(m,t,"with",fe);return le(m,t,"in resolve() call"),new Promise((Y,V)=>{const z={command:"resolve",path:a,key:e,pluginName:s};if(l!=null&&(z.pluginName=l),v!=null&&(z.importer=v),b!=null&&(z.namespace=b),j!=null&&(z.resolveDir=j),M!=null)z.kind=M;else throw new Error('Must specify "kind" when calling "resolve"');A!=null&&(z.pluginData=u.store(A)),R!=null&&(z.with=jt(R,"with")),n(c,z,(de,se)=>{de!==null?V(new Error(de)):Y({errors:ke(se.errors,u),warnings:ke(se.warnings,u),path:se.path,external:se.external,sideEffects:se.sideEffects,namespace:se.namespace,suffix:se.suffix,pluginData:u.load(se.pluginData)})})})},onStart(a){let m='This error came from the "onStart" callback registered here:',t=Ue(new Error(m),g,"onStart");F.push({name:s,callback:a,note:t}),d.onStart=!0},onEnd(a){let m='This error came from the "onEnd" callback registered here:',t=Ue(new Error(m),g,"onEnd");L.push({name:s,callback:a,note:t}),d.onEnd=!0},onResolve(a,m){let t='This error came from the "onResolve" callback registered here:',l=Ue(new Error(t),g,"onResolve"),v={},b=i(a,v,"filter",Re),j=i(a,v,"namespace",C);if(le(a,v,`in onResolve() call for plugin ${H(s)}`),b==null)throw new Error("onResolve() call is missing a filter");let M=B++;D[M]={name:s,callback:m,note:l},d.onResolve.push({id:M,filter:Ne(b),namespace:j||""})},onLoad(a,m){let t='This error came from the "onLoad" callback registered here:',l=Ue(new Error(t),g,"onLoad"),v={},b=i(a,v,"filter",Re),j=i(a,v,"namespace",C);if(le(a,v,`in onLoad() call for plugin ${H(s)}`),b==null)throw new Error("onLoad() call is missing a filter");let M=B++;q[M]={name:s,callback:m,note:l},d.onLoad.push({id:M,filter:Ne(b),namespace:j||""})},onDispose(a){U.push(a)},esbuild:g.esbuild});E&&(yield E),P.push(d)}catch(o){return{ok:!1,error:o,pluginName:s}}}w["on-start"]=(y,f)=>ie(void 0,null,function*(){u.clear();let s={errors:[],warnings:[]};yield Promise.all(F.map(o=>ie(void 0,[o],function*({name:d,callback:x,note:E}){try{let a=yield x();if(a!=null){if(typeof a!="object")throw new Error(`Expected onStart() callback in plugin ${H(d)} to return an object`);let m={},t=i(a,m,"errors",re),l=i(a,m,"warnings",re);le(a,m,`from onStart() callback in plugin ${H(d)}`),t!=null&&s.errors.push(...we(t,"errors",u,d,void 0)),l!=null&&s.warnings.push(...we(l,"warnings",u,d,void 0))}}catch(a){s.errors.push(be(a,g,u,E&&E(),d))}}))),r(y,s)}),w["on-resolve"]=(y,f)=>ie(void 0,null,function*(){let s={},o="",d,x;for(let E of f.ids)try{({name:o,callback:d,note:x}=D[E]);let a=yield d({path:f.path,importer:f.importer,namespace:f.namespace,resolveDir:f.resolveDir,kind:f.kind,pluginData:u.load(f.pluginData),with:f.with});if(a!=null){if(typeof a!="object")throw new Error(`Expected onResolve() callback in plugin ${H(o)} to return an object`);let m={},t=i(a,m,"pluginName",C),l=i(a,m,"path",C),v=i(a,m,"namespace",C),b=i(a,m,"suffix",C),j=i(a,m,"external",Z),M=i(a,m,"sideEffects",Z),A=i(a,m,"pluginData",Ie),R=i(a,m,"errors",re),Y=i(a,m,"warnings",re),V=i(a,m,"watchFiles",re),z=i(a,m,"watchDirs",re);le(a,m,`from onResolve() callback in plugin ${H(o)}`),s.id=E,t!=null&&(s.pluginName=t),l!=null&&(s.path=l),v!=null&&(s.namespace=v),b!=null&&(s.suffix=b),j!=null&&(s.external=j),M!=null&&(s.sideEffects=M),A!=null&&(s.pluginData=u.store(A)),R!=null&&(s.errors=we(R,"errors",u,o,void 0)),Y!=null&&(s.warnings=we(Y,"warnings",u,o,void 0)),V!=null&&(s.watchFiles=Me(V,"watchFiles")),z!=null&&(s.watchDirs=Me(z,"watchDirs"));break}}catch(a){s={id:E,errors:[be(a,g,u,x&&x(),o)]};break}r(y,s)}),w["on-load"]=(y,f)=>ie(void 0,null,function*(){let s={},o="",d,x;for(let E of f.ids)try{({name:o,callback:d,note:x}=q[E]);let a=yield d({path:f.path,namespace:f.namespace,suffix:f.suffix,pluginData:u.load(f.pluginData),with:f.with});if(a!=null){if(typeof a!="object")throw new Error(`Expected onLoad() callback in plugin ${H(o)} to return an object`);let m={},t=i(a,m,"pluginName",C),l=i(a,m,"contents",Ke),v=i(a,m,"resolveDir",C),b=i(a,m,"pluginData",Ie),j=i(a,m,"loader",C),M=i(a,m,"errors",re),A=i(a,m,"warnings",re),R=i(a,m,"watchFiles",re),Y=i(a,m,"watchDirs",re);le(a,m,`from onLoad() callback in plugin ${H(o)}`),s.id=E,t!=null&&(s.pluginName=t),l instanceof Uint8Array?s.contents=l:l!=null&&(s.contents=K(l)),v!=null&&(s.resolveDir=v),b!=null&&(s.pluginData=u.store(b)),j!=null&&(s.loader=j),M!=null&&(s.errors=we(M,"errors",u,o,void 0)),A!=null&&(s.warnings=we(A,"warnings",u,o,void 0)),R!=null&&(s.watchFiles=Me(R,"watchFiles")),Y!=null&&(s.watchDirs=Me(Y,"watchDirs"));break}}catch(a){s={id:E,errors:[be(a,g,u,x&&x(),o)]};break}r(y,s)});let T=(y,f)=>f([],[]);L.length>0&&(T=(y,f)=>{ie(void 0,null,function*(){const s=[],o=[];for(const{name:d,callback:x,note:E}of L){let a,m;try{const t=yield x(y);if(t!=null){if(typeof t!="object")throw new Error(`Expected onEnd() callback in plugin ${H(d)} to return an object`);let l={},v=i(t,l,"errors",re),b=i(t,l,"warnings",re);le(t,l,`from onEnd() callback in plugin ${H(d)}`),v!=null&&(a=we(v,"errors",u,d,void 0)),b!=null&&(m=we(b,"warnings",u,d,void 0))}}catch(t){a=[be(t,g,u,E&&E(),d)]}if(a){s.push(...a);try{y.errors.push(...a)}catch{}}if(m){o.push(...m);try{y.warnings.push(...m)}catch{}}}f(s,o)})});let $=()=>{for(const y of U)setTimeout(()=>y(),0)};return ne=!0,{ok:!0,requestPlugins:P,runOnEndCallbacks:T,scheduleOnDisposeCallbacks:$}});function tt(){const e=new Map;let n=0;return{clear(){e.clear()},load(r){return e.get(r)},store(r){if(r===void 0)return-1;const c=n++;return e.set(c,r),c}}}function Ue(e,n,r){let c,g=!1;return()=>{if(g)return c;g=!0;try{let w=(e.stack+"").split(`
`);w.splice(1,1);let h=nt(n,w,r);if(h)return c={text:e.message,location:h},c}catch{}}}function be(e,n,r,c,g){let w="Internal error",h=null;try{w=(e&&e.message||e)+""}catch{}try{h=nt(n,(e.stack+"").split(`
`),"")}catch{}return{id:"",pluginName:g,text:w,location:h,notes:c?[c]:[],detail:r?r.store(e):-1}}function nt(e,n,r){let c="    at ";if(e.readFileSync&&!n[0].startsWith(c)&&n[1].startsWith(c))for(let g=1;g<n.length;g++){let w=n[g];if(w.startsWith(c))for(w=w.slice(c.length);;){let h=/^(?:new |async )?\S+ \((.*)\)$/.exec(w);if(h){w=h[1];continue}if(h=/^eval at \S+ \((.*)\)(?:, \S+:\d+:\d+)?$/.exec(w),h){w=h[1];continue}if(h=/^(\S+):(\d+):(\d+)$/.exec(w),h){let S;try{S=e.readFileSync(h[1],"utf8")}catch{break}let u=S.split(/\r\n|\r|\n|\u2028|\u2029/)[+h[2]-1]||"",F=+h[3]-1,L=u.slice(F,F+r.length)===r?r.length:0;return{file:h[1],namespace:"file",line:+h[2],column:K(u.slice(0,F)).length,length:K(u.slice(F,F+L)).length,lineText:u+`
`+n.slice(1).join(`
`),suggestion:""}}break}}return null}function je(e,n,r){let c=5;e+=n.length<1?"":` with ${n.length} error${n.length<2?"":"s"}:`+n.slice(0,c+1).map((w,h)=>{if(h===c)return`
...`;if(!w.location)return`
error: ${w.text}`;let{file:S,line:u,column:F}=w.location,L=w.pluginName?`[plugin: ${w.pluginName}] `:"";return`
${S}:${u}:${F}: ERROR: ${L}${w.text}`}).join("");let g=new Error(e);for(const[w,h]of[["errors",n],["warnings",r]])Object.defineProperty(g,w,{configurable:!0,enumerable:!0,get:()=>h,set:S=>Object.defineProperty(g,w,{configurable:!0,enumerable:!0,value:S})});return g}function ke(e,n){for(const r of e)r.detail=n.load(r.detail);return e}function rt(e,n,r){if(e==null)return null;let c={},g=i(e,c,"file",C),w=i(e,c,"namespace",C),h=i(e,c,"line",Ee),S=i(e,c,"column",Ee),u=i(e,c,"length",Ee),F=i(e,c,"lineText",C),L=i(e,c,"suggestion",C);if(le(e,c,n),F){const D=F.slice(0,(S&&S>0?S:0)+(u&&u>0?u:0)+(r&&r>0?r:80));!/[\x7F-\uFFFF]/.test(D)&&!/\n/.test(F)&&(F=D)}return{file:g||"",namespace:w||"",line:h||0,column:S||0,length:u||0,lineText:F||"",suggestion:L||""}}function we(e,n,r,c,g){let w=[],h=0;for(const S of e){let u={},F=i(S,u,"id",C),L=i(S,u,"pluginName",C),D=i(S,u,"text",C),q=i(S,u,"location",Xe),U=i(S,u,"notes",re),B=i(S,u,"detail",Ie),W=`in element ${h} of "${n}"`;le(S,u,W);let P=[];if(U)for(const ne of U){let T={},$=i(ne,T,"text",C),y=i(ne,T,"location",Xe);le(ne,T,W),P.push({text:$||"",location:rt(y,W,g)})}w.push({id:F||"",pluginName:L||c,text:D||"",location:rt(q,W,g),notes:P,detail:r?r.store(B):-1}),h++}return w}function Me(e,n){const r=[];for(const c of e){if(typeof c!="string")throw new Error(`${H(n)} must be an array of strings`);r.push(c)}return r}function jt(e,n){const r=Object.create(null);for(const c in e){const g=e[c];if(typeof g!="string")throw new Error(`key ${H(c)} in object ${H(n)} must be a string`);r[c]=g}return r}function Ot({path:e,contents:n,hash:r}){let c=null;return{path:e,contents:n,hash:r,get text(){const g=this.contents;return(c===null||g!==n)&&(n=g,c=pe(g)),c}}}function Ne(e){let n=e.source;return e.flags&&(n=`(?${e.flags})${n}`),n}var Pt="0.25.2",Ct=e=>Oe().build(e),Dt=e=>Oe().context(e),It=(e,n)=>Oe().transform(e,n),Rt=(e,n)=>Oe().formatMessages(e,n),At=(e,n)=>Oe().analyzeMetafile(e,n),Ut=()=>{throw new Error('The "buildSync" API only works in node')},Mt=()=>{throw new Error('The "transformSync" API only works in node')},Nt=()=>{throw new Error('The "formatMessagesSync" API only works in node')},Ft=()=>{throw new Error('The "analyzeMetafileSync" API only works in node')},Lt=()=>(Fe&&Fe(),Promise.resolve()),xe,Fe,Le,Oe=()=>{if(Le)return Le;throw xe?new Error('You need to wait for the promise returned from "initialize" to be resolved before calling this'):new Error('You need to call "initialize" before calling this')},Vt=e=>{e=_t(e||{});let n=e.wasmURL,r=e.wasmModule,c=e.worker!==!1;if(!n&&!r)throw new Error('Must provide either the "wasmURL" option or the "wasmModule" option');if(xe)throw new Error('Cannot call "initialize" more than once');return xe=Bt(n||"",r,c),xe.catch(()=>{xe=void 0}),xe},Bt=(e,n,r)=>ie(void 0,null,function*(){let c,g;const w=new Promise(D=>g=D);if(r){let D=new Blob([`onmessage=((postMessage) => {
      // Copyright 2018 The Go Authors. All rights reserved.
      // Use of this source code is governed by a BSD-style
      // license that can be found in the LICENSE file.
      var __async = (__this, __arguments, generator) => {
        return new Promise((resolve, reject) => {
          var fulfilled = (value) => {
            try {
              step(generator.next(value));
            } catch (e) {
              reject(e);
            }
          };
          var rejected = (value) => {
            try {
              step(generator.throw(value));
            } catch (e) {
              reject(e);
            }
          };
          var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
          step((generator = generator.apply(__this, __arguments)).next());
        });
      };
      let onmessage;
      let globalThis = {};
      for (let o = self; o; o = Object.getPrototypeOf(o))
        for (let k of Object.getOwnPropertyNames(o))
          if (!(k in globalThis))
            Object.defineProperty(globalThis, k, { get: () => self[k] });
      "use strict";
      (() => {
        const enosys = () => {
          const err = new Error("not implemented");
          err.code = "ENOSYS";
          return err;
        };
        if (!globalThis.fs) {
          let outputBuf = "";
          globalThis.fs = {
            constants: { O_WRONLY: -1, O_RDWR: -1, O_CREAT: -1, O_TRUNC: -1, O_APPEND: -1, O_EXCL: -1 },
            // unused
            writeSync(fd, buf) {
              outputBuf += decoder.decode(buf);
              const nl = outputBuf.lastIndexOf("\\n");
              if (nl != -1) {
                console.log(outputBuf.substring(0, nl));
                outputBuf = outputBuf.substring(nl + 1);
              }
              return buf.length;
            },
            write(fd, buf, offset, length, position, callback) {
              if (offset !== 0 || length !== buf.length || position !== null) {
                callback(enosys());
                return;
              }
              const n = this.writeSync(fd, buf);
              callback(null, n);
            },
            chmod(path, mode, callback) {
              callback(enosys());
            },
            chown(path, uid, gid, callback) {
              callback(enosys());
            },
            close(fd, callback) {
              callback(enosys());
            },
            fchmod(fd, mode, callback) {
              callback(enosys());
            },
            fchown(fd, uid, gid, callback) {
              callback(enosys());
            },
            fstat(fd, callback) {
              callback(enosys());
            },
            fsync(fd, callback) {
              callback(null);
            },
            ftruncate(fd, length, callback) {
              callback(enosys());
            },
            lchown(path, uid, gid, callback) {
              callback(enosys());
            },
            link(path, link, callback) {
              callback(enosys());
            },
            lstat(path, callback) {
              callback(enosys());
            },
            mkdir(path, perm, callback) {
              callback(enosys());
            },
            open(path, flags, mode, callback) {
              callback(enosys());
            },
            read(fd, buffer, offset, length, position, callback) {
              callback(enosys());
            },
            readdir(path, callback) {
              callback(enosys());
            },
            readlink(path, callback) {
              callback(enosys());
            },
            rename(from, to, callback) {
              callback(enosys());
            },
            rmdir(path, callback) {
              callback(enosys());
            },
            stat(path, callback) {
              callback(enosys());
            },
            symlink(path, link, callback) {
              callback(enosys());
            },
            truncate(path, length, callback) {
              callback(enosys());
            },
            unlink(path, callback) {
              callback(enosys());
            },
            utimes(path, atime, mtime, callback) {
              callback(enosys());
            }
          };
        }
        if (!globalThis.process) {
          globalThis.process = {
            getuid() {
              return -1;
            },
            getgid() {
              return -1;
            },
            geteuid() {
              return -1;
            },
            getegid() {
              return -1;
            },
            getgroups() {
              throw enosys();
            },
            pid: -1,
            ppid: -1,
            umask() {
              throw enosys();
            },
            cwd() {
              throw enosys();
            },
            chdir() {
              throw enosys();
            }
          };
        }
        if (!globalThis.crypto) {
          throw new Error("globalThis.crypto is not available, polyfill required (crypto.getRandomValues only)");
        }
        if (!globalThis.performance) {
          throw new Error("globalThis.performance is not available, polyfill required (performance.now only)");
        }
        if (!globalThis.TextEncoder) {
          throw new Error("globalThis.TextEncoder is not available, polyfill required");
        }
        if (!globalThis.TextDecoder) {
          throw new Error("globalThis.TextDecoder is not available, polyfill required");
        }
        const encoder = new TextEncoder("utf-8");
        const decoder = new TextDecoder("utf-8");
        globalThis.Go = class {
          constructor() {
            this.argv = ["js"];
            this.env = {};
            this.exit = (code) => {
              if (code !== 0) {
                console.warn("exit code:", code);
              }
            };
            this._exitPromise = new Promise((resolve) => {
              this._resolveExitPromise = resolve;
            });
            this._pendingEvent = null;
            this._scheduledTimeouts = /* @__PURE__ */ new Map();
            this._nextCallbackTimeoutID = 1;
            const setInt64 = (addr, v) => {
              this.mem.setUint32(addr + 0, v, true);
              this.mem.setUint32(addr + 4, Math.floor(v / 4294967296), true);
            };
            const setInt32 = (addr, v) => {
              this.mem.setUint32(addr + 0, v, true);
            };
            const getInt64 = (addr) => {
              const low = this.mem.getUint32(addr + 0, true);
              const high = this.mem.getInt32(addr + 4, true);
              return low + high * 4294967296;
            };
            const loadValue = (addr) => {
              const f = this.mem.getFloat64(addr, true);
              if (f === 0) {
                return void 0;
              }
              if (!isNaN(f)) {
                return f;
              }
              const id = this.mem.getUint32(addr, true);
              return this._values[id];
            };
            const storeValue = (addr, v) => {
              const nanHead = 2146959360;
              if (typeof v === "number" && v !== 0) {
                if (isNaN(v)) {
                  this.mem.setUint32(addr + 4, nanHead, true);
                  this.mem.setUint32(addr, 0, true);
                  return;
                }
                this.mem.setFloat64(addr, v, true);
                return;
              }
              if (v === void 0) {
                this.mem.setFloat64(addr, 0, true);
                return;
              }
              let id = this._ids.get(v);
              if (id === void 0) {
                id = this._idPool.pop();
                if (id === void 0) {
                  id = this._values.length;
                }
                this._values[id] = v;
                this._goRefCounts[id] = 0;
                this._ids.set(v, id);
              }
              this._goRefCounts[id]++;
              let typeFlag = 0;
              switch (typeof v) {
                case "object":
                  if (v !== null) {
                    typeFlag = 1;
                  }
                  break;
                case "string":
                  typeFlag = 2;
                  break;
                case "symbol":
                  typeFlag = 3;
                  break;
                case "function":
                  typeFlag = 4;
                  break;
              }
              this.mem.setUint32(addr + 4, nanHead | typeFlag, true);
              this.mem.setUint32(addr, id, true);
            };
            const loadSlice = (addr) => {
              const array = getInt64(addr + 0);
              const len = getInt64(addr + 8);
              return new Uint8Array(this._inst.exports.mem.buffer, array, len);
            };
            const loadSliceOfValues = (addr) => {
              const array = getInt64(addr + 0);
              const len = getInt64(addr + 8);
              const a = new Array(len);
              for (let i = 0; i < len; i++) {
                a[i] = loadValue(array + i * 8);
              }
              return a;
            };
            const loadString = (addr) => {
              const saddr = getInt64(addr + 0);
              const len = getInt64(addr + 8);
              return decoder.decode(new DataView(this._inst.exports.mem.buffer, saddr, len));
            };
            const timeOrigin = Date.now() - performance.now();
            this.importObject = {
              _gotest: {
                add: (a, b) => a + b
              },
              gojs: {
                // Go's SP does not change as long as no Go code is running. Some operations (e.g. calls, getters and setters)
                // may synchronously trigger a Go event handler. This makes Go code get executed in the middle of the imported
                // function. A goroutine can switch to a new stack if the current stack is too small (see morestack function).
                // This changes the SP, thus we have to update the SP used by the imported function.
                // func wasmExit(code int32)
                "runtime.wasmExit": (sp) => {
                  sp >>>= 0;
                  const code = this.mem.getInt32(sp + 8, true);
                  this.exited = true;
                  delete this._inst;
                  delete this._values;
                  delete this._goRefCounts;
                  delete this._ids;
                  delete this._idPool;
                  this.exit(code);
                },
                // func wasmWrite(fd uintptr, p unsafe.Pointer, n int32)
                "runtime.wasmWrite": (sp) => {
                  sp >>>= 0;
                  const fd = getInt64(sp + 8);
                  const p = getInt64(sp + 16);
                  const n = this.mem.getInt32(sp + 24, true);
                  globalThis.fs.writeSync(fd, new Uint8Array(this._inst.exports.mem.buffer, p, n));
                },
                // func resetMemoryDataView()
                "runtime.resetMemoryDataView": (sp) => {
                  sp >>>= 0;
                  this.mem = new DataView(this._inst.exports.mem.buffer);
                },
                // func nanotime1() int64
                "runtime.nanotime1": (sp) => {
                  sp >>>= 0;
                  setInt64(sp + 8, (timeOrigin + performance.now()) * 1e6);
                },
                // func walltime() (sec int64, nsec int32)
                "runtime.walltime": (sp) => {
                  sp >>>= 0;
                  const msec = (/* @__PURE__ */ new Date()).getTime();
                  setInt64(sp + 8, msec / 1e3);
                  this.mem.setInt32(sp + 16, msec % 1e3 * 1e6, true);
                },
                // func scheduleTimeoutEvent(delay int64) int32
                "runtime.scheduleTimeoutEvent": (sp) => {
                  sp >>>= 0;
                  const id = this._nextCallbackTimeoutID;
                  this._nextCallbackTimeoutID++;
                  this._scheduledTimeouts.set(id, setTimeout(
                    () => {
                      this._resume();
                      while (this._scheduledTimeouts.has(id)) {
                        console.warn("scheduleTimeoutEvent: missed timeout event");
                        this._resume();
                      }
                    },
                    getInt64(sp + 8)
                  ));
                  this.mem.setInt32(sp + 16, id, true);
                },
                // func clearTimeoutEvent(id int32)
                "runtime.clearTimeoutEvent": (sp) => {
                  sp >>>= 0;
                  const id = this.mem.getInt32(sp + 8, true);
                  clearTimeout(this._scheduledTimeouts.get(id));
                  this._scheduledTimeouts.delete(id);
                },
                // func getRandomData(r []byte)
                "runtime.getRandomData": (sp) => {
                  sp >>>= 0;
                  crypto.getRandomValues(loadSlice(sp + 8));
                },
                // func finalizeRef(v ref)
                "syscall/js.finalizeRef": (sp) => {
                  sp >>>= 0;
                  const id = this.mem.getUint32(sp + 8, true);
                  this._goRefCounts[id]--;
                  if (this._goRefCounts[id] === 0) {
                    const v = this._values[id];
                    this._values[id] = null;
                    this._ids.delete(v);
                    this._idPool.push(id);
                  }
                },
                // func stringVal(value string) ref
                "syscall/js.stringVal": (sp) => {
                  sp >>>= 0;
                  storeValue(sp + 24, loadString(sp + 8));
                },
                // func valueGet(v ref, p string) ref
                "syscall/js.valueGet": (sp) => {
                  sp >>>= 0;
                  const result = Reflect.get(loadValue(sp + 8), loadString(sp + 16));
                  sp = this._inst.exports.getsp() >>> 0;
                  storeValue(sp + 32, result);
                },
                // func valueSet(v ref, p string, x ref)
                "syscall/js.valueSet": (sp) => {
                  sp >>>= 0;
                  Reflect.set(loadValue(sp + 8), loadString(sp + 16), loadValue(sp + 32));
                },
                // func valueDelete(v ref, p string)
                "syscall/js.valueDelete": (sp) => {
                  sp >>>= 0;
                  Reflect.deleteProperty(loadValue(sp + 8), loadString(sp + 16));
                },
                // func valueIndex(v ref, i int) ref
                "syscall/js.valueIndex": (sp) => {
                  sp >>>= 0;
                  storeValue(sp + 24, Reflect.get(loadValue(sp + 8), getInt64(sp + 16)));
                },
                // valueSetIndex(v ref, i int, x ref)
                "syscall/js.valueSetIndex": (sp) => {
                  sp >>>= 0;
                  Reflect.set(loadValue(sp + 8), getInt64(sp + 16), loadValue(sp + 24));
                },
                // func valueCall(v ref, m string, args []ref) (ref, bool)
                "syscall/js.valueCall": (sp) => {
                  sp >>>= 0;
                  try {
                    const v = loadValue(sp + 8);
                    const m = Reflect.get(v, loadString(sp + 16));
                    const args = loadSliceOfValues(sp + 32);
                    const result = Reflect.apply(m, v, args);
                    sp = this._inst.exports.getsp() >>> 0;
                    storeValue(sp + 56, result);
                    this.mem.setUint8(sp + 64, 1);
                  } catch (err) {
                    sp = this._inst.exports.getsp() >>> 0;
                    storeValue(sp + 56, err);
                    this.mem.setUint8(sp + 64, 0);
                  }
                },
                // func valueInvoke(v ref, args []ref) (ref, bool)
                "syscall/js.valueInvoke": (sp) => {
                  sp >>>= 0;
                  try {
                    const v = loadValue(sp + 8);
                    const args = loadSliceOfValues(sp + 16);
                    const result = Reflect.apply(v, void 0, args);
                    sp = this._inst.exports.getsp() >>> 0;
                    storeValue(sp + 40, result);
                    this.mem.setUint8(sp + 48, 1);
                  } catch (err) {
                    sp = this._inst.exports.getsp() >>> 0;
                    storeValue(sp + 40, err);
                    this.mem.setUint8(sp + 48, 0);
                  }
                },
                // func valueNew(v ref, args []ref) (ref, bool)
                "syscall/js.valueNew": (sp) => {
                  sp >>>= 0;
                  try {
                    const v = loadValue(sp + 8);
                    const args = loadSliceOfValues(sp + 16);
                    const result = Reflect.construct(v, args);
                    sp = this._inst.exports.getsp() >>> 0;
                    storeValue(sp + 40, result);
                    this.mem.setUint8(sp + 48, 1);
                  } catch (err) {
                    sp = this._inst.exports.getsp() >>> 0;
                    storeValue(sp + 40, err);
                    this.mem.setUint8(sp + 48, 0);
                  }
                },
                // func valueLength(v ref) int
                "syscall/js.valueLength": (sp) => {
                  sp >>>= 0;
                  setInt64(sp + 16, parseInt(loadValue(sp + 8).length));
                },
                // valuePrepareString(v ref) (ref, int)
                "syscall/js.valuePrepareString": (sp) => {
                  sp >>>= 0;
                  const str = encoder.encode(String(loadValue(sp + 8)));
                  storeValue(sp + 16, str);
                  setInt64(sp + 24, str.length);
                },
                // valueLoadString(v ref, b []byte)
                "syscall/js.valueLoadString": (sp) => {
                  sp >>>= 0;
                  const str = loadValue(sp + 8);
                  loadSlice(sp + 16).set(str);
                },
                // func valueInstanceOf(v ref, t ref) bool
                "syscall/js.valueInstanceOf": (sp) => {
                  sp >>>= 0;
                  this.mem.setUint8(sp + 24, loadValue(sp + 8) instanceof loadValue(sp + 16) ? 1 : 0);
                },
                // func copyBytesToGo(dst []byte, src ref) (int, bool)
                "syscall/js.copyBytesToGo": (sp) => {
                  sp >>>= 0;
                  const dst = loadSlice(sp + 8);
                  const src = loadValue(sp + 32);
                  if (!(src instanceof Uint8Array || src instanceof Uint8ClampedArray)) {
                    this.mem.setUint8(sp + 48, 0);
                    return;
                  }
                  const toCopy = src.subarray(0, dst.length);
                  dst.set(toCopy);
                  setInt64(sp + 40, toCopy.length);
                  this.mem.setUint8(sp + 48, 1);
                },
                // func copyBytesToJS(dst ref, src []byte) (int, bool)
                "syscall/js.copyBytesToJS": (sp) => {
                  sp >>>= 0;
                  const dst = loadValue(sp + 8);
                  const src = loadSlice(sp + 16);
                  if (!(dst instanceof Uint8Array || dst instanceof Uint8ClampedArray)) {
                    this.mem.setUint8(sp + 48, 0);
                    return;
                  }
                  const toCopy = src.subarray(0, dst.length);
                  dst.set(toCopy);
                  setInt64(sp + 40, toCopy.length);
                  this.mem.setUint8(sp + 48, 1);
                },
                "debug": (value) => {
                  console.log(value);
                }
              }
            };
          }
          run(instance) {
            return __async(this, null, function* () {
              if (!(instance instanceof WebAssembly.Instance)) {
                throw new Error("Go.run: WebAssembly.Instance expected");
              }
              this._inst = instance;
              this.mem = new DataView(this._inst.exports.mem.buffer);
              this._values = [
                // JS values that Go currently has references to, indexed by reference id
                NaN,
                0,
                null,
                true,
                false,
                globalThis,
                this
              ];
              this._goRefCounts = new Array(this._values.length).fill(Infinity);
              this._ids = /* @__PURE__ */ new Map([
                // mapping from JS values to reference ids
                [0, 1],
                [null, 2],
                [true, 3],
                [false, 4],
                [globalThis, 5],
                [this, 6]
              ]);
              this._idPool = [];
              this.exited = false;
              let offset = 4096;
              const strPtr = (str) => {
                const ptr = offset;
                const bytes = encoder.encode(str + "\\0");
                new Uint8Array(this.mem.buffer, offset, bytes.length).set(bytes);
                offset += bytes.length;
                if (offset % 8 !== 0) {
                  offset += 8 - offset % 8;
                }
                return ptr;
              };
              const argc = this.argv.length;
              const argvPtrs = [];
              this.argv.forEach((arg) => {
                argvPtrs.push(strPtr(arg));
              });
              argvPtrs.push(0);
              const keys = Object.keys(this.env).sort();
              keys.forEach((key) => {
                argvPtrs.push(strPtr(\`\${key}=\${this.env[key]}\`));
              });
              argvPtrs.push(0);
              const argv = offset;
              argvPtrs.forEach((ptr) => {
                this.mem.setUint32(offset, ptr, true);
                this.mem.setUint32(offset + 4, 0, true);
                offset += 8;
              });
              const wasmMinDataAddr = 4096 + 8192;
              if (offset >= wasmMinDataAddr) {
                throw new Error("total length of command line and environment variables exceeds limit");
              }
              this._inst.exports.run(argc, argv);
              if (this.exited) {
                this._resolveExitPromise();
              }
              yield this._exitPromise;
            });
          }
          _resume() {
            if (this.exited) {
              throw new Error("Go program has already exited");
            }
            this._inst.exports.resume();
            if (this.exited) {
              this._resolveExitPromise();
            }
          }
          _makeFuncWrapper(id) {
            const go = this;
            return function() {
              const event = { id, this: this, args: arguments };
              go._pendingEvent = event;
              go._resume();
              return event.result;
            };
          }
        };
      })();
      onmessage = ({ data: wasm }) => {
        let decoder = new TextDecoder();
        let fs = globalThis.fs;
        let stderr = "";
        fs.writeSync = (fd, buffer) => {
          if (fd === 1) {
            postMessage(buffer);
          } else if (fd === 2) {
            stderr += decoder.decode(buffer);
            let parts = stderr.split("\\n");
            if (parts.length > 1) console.log(parts.slice(0, -1).join("\\n"));
            stderr = parts[parts.length - 1];
          } else {
            throw new Error("Bad write");
          }
          return buffer.length;
        };
        let stdin = [];
        let resumeStdin;
        let stdinPos = 0;
        onmessage = ({ data }) => {
          if (data.length > 0) {
            stdin.push(data);
            if (resumeStdin) resumeStdin();
          }
          return go;
        };
        fs.read = (fd, buffer, offset, length, position, callback) => {
          if (fd !== 0 || offset !== 0 || length !== buffer.length || position !== null) {
            throw new Error("Bad read");
          }
          if (stdin.length === 0) {
            resumeStdin = () => fs.read(fd, buffer, offset, length, position, callback);
            return;
          }
          let first = stdin[0];
          let count = Math.max(0, Math.min(length, first.length - stdinPos));
          buffer.set(first.subarray(stdinPos, stdinPos + count), offset);
          stdinPos += count;
          if (stdinPos === first.length) {
            stdin.shift();
            stdinPos = 0;
          }
          callback(null, count);
        };
        let go = new globalThis.Go();
        go.argv = ["", \`--service=\${"0.25.2"}\`];
        tryToInstantiateModule(wasm, go).then(
          (instance) => {
            postMessage(null);
            go.run(instance);
          },
          (error) => {
            postMessage(error);
          }
        );
        return go;
      };
      function tryToInstantiateModule(wasm, go) {
        return __async(this, null, function* () {
          if (wasm instanceof WebAssembly.Module) {
            return WebAssembly.instantiate(wasm, go.importObject);
          }
          const res = yield fetch(wasm);
          if (!res.ok) throw new Error(\`Failed to download \${JSON.stringify(wasm)}\`);
          if ("instantiateStreaming" in WebAssembly && /^application\\/wasm($|;)/i.test(res.headers.get("Content-Type") || "")) {
            const result2 = yield WebAssembly.instantiateStreaming(res, go.importObject);
            return result2.instance;
          }
          const bytes = yield res.arrayBuffer();
          const result = yield WebAssembly.instantiate(bytes, go.importObject);
          return result.instance;
        });
      }
      return (m) => onmessage(m);
    })(postMessage)`],{type:"text/javascript"});c=new Worker(URL.createObjectURL(D))}else{let D=(U=>{var B=(T,$,y)=>new Promise((f,s)=>{var o=E=>{try{x(y.next(E))}catch(a){s(a)}},d=E=>{try{x(y.throw(E))}catch(a){s(a)}},x=E=>E.done?f(E.value):Promise.resolve(E.value).then(o,d);x((y=y.apply(T,$)).next())});let W,P={};for(let T=self;T;T=Object.getPrototypeOf(T))for(let $ of Object.getOwnPropertyNames(T))$ in P||Object.defineProperty(P,$,{get:()=>self[$]});(()=>{const T=()=>{const f=new Error("not implemented");return f.code="ENOSYS",f};if(!P.fs){let f="";P.fs={constants:{O_WRONLY:-1,O_RDWR:-1,O_CREAT:-1,O_TRUNC:-1,O_APPEND:-1,O_EXCL:-1},writeSync(s,o){f+=y.decode(o);const d=f.lastIndexOf(`
`);return d!=-1&&(console.log(f.substring(0,d)),f=f.substring(d+1)),o.length},write(s,o,d,x,E,a){if(d!==0||x!==o.length||E!==null){a(T());return}const m=this.writeSync(s,o);a(null,m)},chmod(s,o,d){d(T())},chown(s,o,d,x){x(T())},close(s,o){o(T())},fchmod(s,o,d){d(T())},fchown(s,o,d,x){x(T())},fstat(s,o){o(T())},fsync(s,o){o(null)},ftruncate(s,o,d){d(T())},lchown(s,o,d,x){x(T())},link(s,o,d){d(T())},lstat(s,o){o(T())},mkdir(s,o,d){d(T())},open(s,o,d,x){x(T())},read(s,o,d,x,E,a){a(T())},readdir(s,o){o(T())},readlink(s,o){o(T())},rename(s,o,d){d(T())},rmdir(s,o){o(T())},stat(s,o){o(T())},symlink(s,o,d){d(T())},truncate(s,o,d){d(T())},unlink(s,o){o(T())},utimes(s,o,d,x){x(T())}}}if(P.process||(P.process={getuid(){return-1},getgid(){return-1},geteuid(){return-1},getegid(){return-1},getgroups(){throw T()},pid:-1,ppid:-1,umask(){throw T()},cwd(){throw T()},chdir(){throw T()}}),!P.crypto)throw new Error("globalThis.crypto is not available, polyfill required (crypto.getRandomValues only)");if(!P.performance)throw new Error("globalThis.performance is not available, polyfill required (performance.now only)");if(!P.TextEncoder)throw new Error("globalThis.TextEncoder is not available, polyfill required");if(!P.TextDecoder)throw new Error("globalThis.TextDecoder is not available, polyfill required");const $=new TextEncoder("utf-8"),y=new TextDecoder("utf-8");P.Go=class{constructor(){this.argv=["js"],this.env={},this.exit=t=>{t!==0&&console.warn("exit code:",t)},this._exitPromise=new Promise(t=>{this._resolveExitPromise=t}),this._pendingEvent=null,this._scheduledTimeouts=new Map,this._nextCallbackTimeoutID=1;const f=(t,l)=>{this.mem.setUint32(t+0,l,!0),this.mem.setUint32(t+4,Math.floor(l/4294967296),!0)},s=t=>{const l=this.mem.getUint32(t+0,!0),v=this.mem.getInt32(t+4,!0);return l+v*4294967296},o=t=>{const l=this.mem.getFloat64(t,!0);if(l===0)return;if(!isNaN(l))return l;const v=this.mem.getUint32(t,!0);return this._values[v]},d=(t,l)=>{if(typeof l=="number"&&l!==0){if(isNaN(l)){this.mem.setUint32(t+4,2146959360,!0),this.mem.setUint32(t,0,!0);return}this.mem.setFloat64(t,l,!0);return}if(l===void 0){this.mem.setFloat64(t,0,!0);return}let b=this._ids.get(l);b===void 0&&(b=this._idPool.pop(),b===void 0&&(b=this._values.length),this._values[b]=l,this._goRefCounts[b]=0,this._ids.set(l,b)),this._goRefCounts[b]++;let j=0;switch(typeof l){case"object":l!==null&&(j=1);break;case"string":j=2;break;case"symbol":j=3;break;case"function":j=4;break}this.mem.setUint32(t+4,2146959360|j,!0),this.mem.setUint32(t,b,!0)},x=t=>{const l=s(t+0),v=s(t+8);return new Uint8Array(this._inst.exports.mem.buffer,l,v)},E=t=>{const l=s(t+0),v=s(t+8),b=new Array(v);for(let j=0;j<v;j++)b[j]=o(l+j*8);return b},a=t=>{const l=s(t+0),v=s(t+8);return y.decode(new DataView(this._inst.exports.mem.buffer,l,v))},m=Date.now()-performance.now();this.importObject={_gotest:{add:(t,l)=>t+l},gojs:{"runtime.wasmExit":t=>{t>>>=0;const l=this.mem.getInt32(t+8,!0);this.exited=!0,delete this._inst,delete this._values,delete this._goRefCounts,delete this._ids,delete this._idPool,this.exit(l)},"runtime.wasmWrite":t=>{t>>>=0;const l=s(t+8),v=s(t+16),b=this.mem.getInt32(t+24,!0);P.fs.writeSync(l,new Uint8Array(this._inst.exports.mem.buffer,v,b))},"runtime.resetMemoryDataView":t=>{this.mem=new DataView(this._inst.exports.mem.buffer)},"runtime.nanotime1":t=>{t>>>=0,f(t+8,(m+performance.now())*1e6)},"runtime.walltime":t=>{t>>>=0;const l=new Date().getTime();f(t+8,l/1e3),this.mem.setInt32(t+16,l%1e3*1e6,!0)},"runtime.scheduleTimeoutEvent":t=>{t>>>=0;const l=this._nextCallbackTimeoutID;this._nextCallbackTimeoutID++,this._scheduledTimeouts.set(l,setTimeout(()=>{for(this._resume();this._scheduledTimeouts.has(l);)console.warn("scheduleTimeoutEvent: missed timeout event"),this._resume()},s(t+8))),this.mem.setInt32(t+16,l,!0)},"runtime.clearTimeoutEvent":t=>{t>>>=0;const l=this.mem.getInt32(t+8,!0);clearTimeout(this._scheduledTimeouts.get(l)),this._scheduledTimeouts.delete(l)},"runtime.getRandomData":t=>{t>>>=0,crypto.getRandomValues(x(t+8))},"syscall/js.finalizeRef":t=>{t>>>=0;const l=this.mem.getUint32(t+8,!0);if(this._goRefCounts[l]--,this._goRefCounts[l]===0){const v=this._values[l];this._values[l]=null,this._ids.delete(v),this._idPool.push(l)}},"syscall/js.stringVal":t=>{t>>>=0,d(t+24,a(t+8))},"syscall/js.valueGet":t=>{t>>>=0;const l=Reflect.get(o(t+8),a(t+16));t=this._inst.exports.getsp()>>>0,d(t+32,l)},"syscall/js.valueSet":t=>{t>>>=0,Reflect.set(o(t+8),a(t+16),o(t+32))},"syscall/js.valueDelete":t=>{t>>>=0,Reflect.deleteProperty(o(t+8),a(t+16))},"syscall/js.valueIndex":t=>{t>>>=0,d(t+24,Reflect.get(o(t+8),s(t+16)))},"syscall/js.valueSetIndex":t=>{t>>>=0,Reflect.set(o(t+8),s(t+16),o(t+24))},"syscall/js.valueCall":t=>{t>>>=0;try{const l=o(t+8),v=Reflect.get(l,a(t+16)),b=E(t+32),j=Reflect.apply(v,l,b);t=this._inst.exports.getsp()>>>0,d(t+56,j),this.mem.setUint8(t+64,1)}catch(l){t=this._inst.exports.getsp()>>>0,d(t+56,l),this.mem.setUint8(t+64,0)}},"syscall/js.valueInvoke":t=>{t>>>=0;try{const l=o(t+8),v=E(t+16),b=Reflect.apply(l,void 0,v);t=this._inst.exports.getsp()>>>0,d(t+40,b),this.mem.setUint8(t+48,1)}catch(l){t=this._inst.exports.getsp()>>>0,d(t+40,l),this.mem.setUint8(t+48,0)}},"syscall/js.valueNew":t=>{t>>>=0;try{const l=o(t+8),v=E(t+16),b=Reflect.construct(l,v);t=this._inst.exports.getsp()>>>0,d(t+40,b),this.mem.setUint8(t+48,1)}catch(l){t=this._inst.exports.getsp()>>>0,d(t+40,l),this.mem.setUint8(t+48,0)}},"syscall/js.valueLength":t=>{t>>>=0,f(t+16,parseInt(o(t+8).length))},"syscall/js.valuePrepareString":t=>{t>>>=0;const l=$.encode(String(o(t+8)));d(t+16,l),f(t+24,l.length)},"syscall/js.valueLoadString":t=>{t>>>=0;const l=o(t+8);x(t+16).set(l)},"syscall/js.valueInstanceOf":t=>{t>>>=0,this.mem.setUint8(t+24,o(t+8)instanceof o(t+16)?1:0)},"syscall/js.copyBytesToGo":t=>{t>>>=0;const l=x(t+8),v=o(t+32);if(!(v instanceof Uint8Array||v instanceof Uint8ClampedArray)){this.mem.setUint8(t+48,0);return}const b=v.subarray(0,l.length);l.set(b),f(t+40,b.length),this.mem.setUint8(t+48,1)},"syscall/js.copyBytesToJS":t=>{t>>>=0;const l=o(t+8),v=x(t+16);if(!(l instanceof Uint8Array||l instanceof Uint8ClampedArray)){this.mem.setUint8(t+48,0);return}const b=v.subarray(0,l.length);l.set(b),f(t+40,b.length),this.mem.setUint8(t+48,1)},debug:t=>{console.log(t)}}}}run(f){return B(this,null,function*(){if(!(f instanceof WebAssembly.Instance))throw new Error("Go.run: WebAssembly.Instance expected");this._inst=f,this.mem=new DataView(this._inst.exports.mem.buffer),this._values=[NaN,0,null,!0,!1,P,this],this._goRefCounts=new Array(this._values.length).fill(1/0),this._ids=new Map([[0,1],[null,2],[!0,3],[!1,4],[P,5],[this,6]]),this._idPool=[],this.exited=!1;let s=4096;const o=t=>{const l=s,v=$.encode(t+"\0");return new Uint8Array(this.mem.buffer,s,v.length).set(v),s+=v.length,s%8!==0&&(s+=8-s%8),l},d=this.argv.length,x=[];this.argv.forEach(t=>{x.push(o(t))}),x.push(0),Object.keys(this.env).sort().forEach(t=>{x.push(o(`${t}=${this.env[t]}`))}),x.push(0);const a=s;if(x.forEach(t=>{this.mem.setUint32(s,t,!0),this.mem.setUint32(s+4,0,!0),s+=8}),s>=12288)throw new Error("total length of command line and environment variables exceeds limit");this._inst.exports.run(d,a),this.exited&&this._resolveExitPromise(),yield this._exitPromise})}_resume(){if(this.exited)throw new Error("Go program has already exited");this._inst.exports.resume(),this.exited&&this._resolveExitPromise()}_makeFuncWrapper(f){const s=this;return function(){const o={id:f,this:this,args:arguments};return s._pendingEvent=o,s._resume(),o.result}}}})(),W=({data:T})=>{let $=new TextDecoder,y=P.fs,f="";y.writeSync=(E,a)=>{if(E===1)U(a);else if(E===2){f+=$.decode(a);let m=f.split(`
`);m.length>1&&console.log(m.slice(0,-1).join(`
`)),f=m[m.length-1]}else throw new Error("Bad write");return a.length};let s=[],o,d=0;W=({data:E})=>(E.length>0&&(s.push(E),o&&o()),x),y.read=(E,a,m,t,l,v)=>{if(E!==0||m!==0||t!==a.length||l!==null)throw new Error("Bad read");if(s.length===0){o=()=>y.read(E,a,m,t,l,v);return}let b=s[0],j=Math.max(0,Math.min(t,b.length-d));a.set(b.subarray(d,d+j),m),d+=j,d===b.length&&(s.shift(),d=0),v(null,j)};let x=new P.Go;return x.argv=["","--service=0.25.2"],ne(T,x).then(E=>{U(null),x.run(E)},E=>{U(E)}),x};function ne(T,$){return B(this,null,function*(){if(T instanceof WebAssembly.Module)return WebAssembly.instantiate(T,$.importObject);const y=yield fetch(T);if(!y.ok)throw new Error(`Failed to download ${JSON.stringify(T)}`);if("instantiateStreaming"in WebAssembly&&/^application\/wasm($|;)/i.test(y.headers.get("Content-Type")||""))return(yield WebAssembly.instantiateStreaming(y,$.importObject)).instance;const f=yield y.arrayBuffer();return(yield WebAssembly.instantiate(f,$.importObject)).instance})}return T=>W(T)})(U=>c.onmessage({data:U})),q;c={onmessage:null,postMessage:U=>setTimeout(()=>{try{q=D({data:U})}catch(B){g(B)}}),terminate(){if(q)for(let U of q._scheduledTimeouts.values())clearTimeout(U)}}}let h,S;const u=new Promise((D,q)=>{h=D,S=q});c.onmessage=({data:D})=>{c.onmessage=({data:q})=>F(q),D?S(D):h()},c.postMessage(n||new URL(e,location.href).toString());let{readFromStdout:F,service:L}=St({writeToStdin(D){c.postMessage(D)},isSync:!1,hasFS:!1,esbuild:Q});yield u,Fe=()=>{c.terminate(),xe=void 0,Fe=void 0,Le=void 0},Le={build:D=>new Promise((q,U)=>{w.then(U),L.buildOrContext({callName:"build",refs:null,options:D,isTTY:!1,defaultWD:"/",callback:(B,W)=>B?U(B):q(W)})}),context:D=>new Promise((q,U)=>{w.then(U),L.buildOrContext({callName:"context",refs:null,options:D,isTTY:!1,defaultWD:"/",callback:(B,W)=>B?U(B):q(W)})}),transform:(D,q)=>new Promise((U,B)=>{w.then(B),L.transform({callName:"transform",refs:null,input:D,options:q||{},isTTY:!1,fs:{readFile(W,P){P(new Error("Internal error"),null)},writeFile(W,P){P(null)}},callback:(W,P)=>W?B(W):U(P)})}),formatMessages:(D,q)=>new Promise((U,B)=>{w.then(B),L.formatMessages({callName:"formatMessages",refs:null,messages:D,options:q,callback:(W,P)=>W?B(W):U(P)})}),analyzeMetafile:(D,q)=>new Promise((U,B)=>{w.then(B),L.analyzeMetafile({callName:"analyzeMetafile",refs:null,metafile:typeof D=="string"?D:JSON.stringify(D),options:q,callback:(W,P)=>W?B(W):U(P)})})}}),Wt=Q})(_)}(We)),We.exports}var ze=qt();let Yt=3,ot=1;class Ge{constructor(p){const I=p.kind_===0?p.content_.length:0,N=p.mtime_.getTime(),G=p.ctime_.getTime();this.dev=1,this.ino=p.inode_,this.mode=p.kind_===0?32768:16384,this.nlink=1,this.uid=1,this.gid=1,this.rdev=0,this.size=I,this.blksize=4096,this.blocks=I+4095&4095,this.atimeMs=N,this.mtimeMs=N,this.ctimeMs=G,this.birthtimeMs=G,this.atime=p.mtime_,this.mtime=p.mtime_,this.ctime=p.ctime_,this.birthtime=p.ctime_}isDirectory(){return this.mode===16384}isFile(){return this.mode===32768}}const Je=Ce("EBADF"),at=Ce("EINVAL"),Ht=Ce("EISDIR"),Xt=Ce("ENOENT"),ct=Ce("ENOTDIR"),Be=new Map,Qt=new TextEncoder,Kt=new TextDecoder,qe=dt();let ut="",Ye,ft;function Zt(_,p,O,I,N){if(_<=2)_===2?pt(p,O,I):Ye(_,p,O,I,N);else throw at}function en(_,p,O,I,N,G){if(_<=2)ft(_,p,O,I,N,G);else{const X=Be.get(_);if(!X)G(Je,0,p);else if(X.entry_.kind_===1)G(Ht,0,p);else{const te=X.entry_.content_;if(N!==null&&N!==-1){const J=te.slice(N,N+I);p.set(J,O),G(null,J.length,p)}else{const J=te.slice(X.offset_,X.offset_+I);X.offset_+=J.length,p.set(J,O),G(null,J.length,p)}}}}function it(_){throw new Error(JSON.stringify(_)+" cannot be both a file and a directory")}function tn(_){qe.children_.clear(),ut="";for(const p in _){const O=mt(ht(p));let I=qe;for(let G=0;G+1<O.length;G++){const X=O[G];let te=I.children_.get(X);te?te.kind_!==1&&it(X):(te=dt(),I.children_.set(X,te)),I=te}const N=O[O.length-1];I.children_.has(N)&&it(N),I.children_.set(N,nn(Qt.encode(_[p])))}}globalThis.fs={get writeSync(){return Zt},set writeSync(_){Ye=_},get read(){return en},set read(_){ft=_},constants:{O_WRONLY:-1,O_RDWR:-1,O_CREAT:-1,O_TRUNC:-1,O_APPEND:-1,O_EXCL:-1},open(_,p,O,I){try{const N=Ve(_),G=Yt++;Be.set(G,{entry_:N,offset_:0}),I(null,G)}catch(N){I(N,null)}},close(_,p){p(Be.delete(_)?null:Je)},write(_,p,O,I,N,G){_<=2?(_===2?pt(p,O,I):Ye(_,p,O,I,N),G(null,I,p)):G(at,0,p)},readdir(_,p){try{const O=Ve(_);if(O.kind_!==1)throw ct;p(null,[...O.children_.keys()])}catch(O){p(O,null)}},stat(_,p){try{const O=Ve(_);p(null,new Ge(O))}catch(O){p(O,null)}},lstat(_,p){try{const O=Ve(_);p(null,new Ge(O))}catch(O){p(O,null)}},fstat(_,p){const O=Be.get(_);O?p(null,new Ge(O.entry_)):p(Je,null)}};function nn(_){const p=new Date;return{kind_:0,inode_:ot++,ctime_:p,mtime_:p,content_:_}}function dt(){const _=new Date;return{kind_:1,inode_:ot++,ctime_:_,mtime_:_,children_:new Map}}function ht(_){_[0]!=="/"&&(_="/"+_);const p=_.split("/");p.shift();let O=0;for(let I=0;I<p.length;I++){const N=p[I];N===".."?O&&O--:N!=="."&&N!==""&&(p[O++]=N)}return p.length=O,"/"+p.join("/")}function mt(_){if(_=ht(_),_==="/")return[];const p=_.split("/");return p.shift(),p}function Ve(_){const p=mt(_);let O=qe;for(let I=0,N=p.length;I<N;I++){const G=O.children_.get(p[I]);if(!G)throw Xt;if(G.kind_===0){if(I+1===N)return G;throw ct}O=G}return O}function Ce(_){const p=new Error(_);return p.code=_,p}function pt(_,p,O){ut+=Kt.decode(p===0&&O===_.length?_:_.slice(p,p+O))}let lt=!1;async function rn(_,p){if(p!="3.5.13")throw new Error(`Vue version ${p} is not supported, use 3.5.13`);const{parse:O,compileTemplate:I,compileScript:N,compileStyle:G}=await import("./vue-DvO6XG5G.js");return{name:"vue",setup(X){X.onResolve({filter:/\.vue\.\d+\.css$/},J=>({path:J.path,namespace:"vue-css"}));const te=new Map;X.onLoad({filter:/.*/,namespace:"vue-css"},J=>({contents:te.get(J.path),loader:"css"})),X.onLoad({filter:/\.vue$/},async J=>{const ie=_[J.path],{descriptor:Q}=O(ie,{filename:J.path}),me=J.path.split("/").pop(),ae=`data-v-${sn(J.path)}`,ce=Q.styles.some(H=>H.scoped);let K="",pe="",_e=[];if(Q.scriptSetup){if(Q.template&&I({source:Q.template.content,filename:J.path,id:me,scoped:ce,scopeId:ce?ae:null,preprocessOptions:{},compilerOptions:{mode:"module"}}),K=N(Q,{id:me,templateOptions:{scoped:ce,scopeId:ce?ae:null},inlineTemplate:!0}).content,ce&&K.lastIndexOf("export default")!==-1){const ve=K.lastIndexOf("}");ve!==-1&&(K=K.slice(0,ve)+`,
  __scopeId: "${ae}"
`+K.slice(ve))}}else Q.script?K=`const _sfc_main = ${Q.script.content.replace("export default","")}`:K="const _sfc_main = {}";if(Q.template&&!Q.scriptSetup&&(pe=I({source:Q.template.content,filename:J.path,id:me,scoped:ce,scopeId:ce?ae:null,preprocessOptions:{},compilerOptions:{mode:"module"}}).code),Q.styles.length>0)for(let H=0;H<Q.styles.length;H++){const ye=Q.styles[H],ve=await G({source:ye.content,filename:J.path,id:ae,scoped:ye.scoped,modules:!!ye.module,preprocessLang:ye.lang,modulesName:ye.module?`style${H}`:void 0}),Te=`${J.path}.${H}.css`;te.set(Te,ve.code),_e.push(`import "${Te}"`)}let $e="";return Q.scriptSetup||($e=`
              ${pe?"_sfc_main.render = render;":""}
              ${ce?`_sfc_main.__scopeId = "${ae}";`:""}
              export default _sfc_main;
            `),{contents:`
            ${_e.join(`
`)}
            
            ${K}
            
            ${pe}
            
            ${$e}
          `.trim(),loader:"js"}})}}}function sn(_){let p=0;if(_.length===0)return p.toString(36);for(let O=0;O<_.length;O++){const I=_.charCodeAt(O);p=(p<<5)-p+I,p=p&p}return p.toString(36).replace("-","_")}async function ln(_,p){if(p!="5.16.1")throw new Error(`Svelte version ${p} is not supported, use 5.16.1`);return await import("./svelte-WbVc7F0A.js"),{name:"svelte",setup(O){O.onLoad({filter:/\.svelte$/},async I=>{let N=({message:te,start:J,end:ie})=>{let Q;if(J&&ie){let me=G.split(/\r\n|\r|\n/g)[J.line-1],ae=J.line===ie.line?ie.column:me.length;Q={file:X,line:J.line,column:J.column,length:ae-J.column,lineText:me}}return{text:te,location:Q}},G=_[I.path],X=I.path;try{let{js:te,warnings:J}=globalThis.svelte.compile(G,{filename:X,dev:!0,css:"injected"});return{contents:te.code+`
//# sourceMappingURL=data:application/json;charset=utf-8;base64,`+btoa(JSON.stringify(te.map)),warnings:J.map(Q=>N({message:Q.message,start:Q.start,end:Q.end}))}}catch(te){return{errors:[N(te)]}}})}}}self.onmessage=async _=>{var N,G;const{id:p,type:O,data:I}=_.data;if(O==="init")lt||(await ze.initialize({wasmURL:"/ui_builder/esbuild.wasm",worker:!1}),lt=!0),postMessage({id:p,success:!0});else if(O==="build")try{tn(I.files);let X=I.files["/package.json"]?JSON.parse(I.files["/package.json"]):{},te=(N=X==null?void 0:X.dependencies)==null?void 0:N.svelte,J=(G=X==null?void 0:X.dependencies)==null?void 0:G.vue,ie=[];te&&(ie=[await ln(I.files,te)]),J&&(ie=[await rn(I.files,J)]);let Q=I.files["/index.tsx"]?["/index.tsx"]:I.files["/index.js"]?["/index.js"]:["/index.ts"];const me=await ze.build({entryPoints:Q,resolveExtensions:[".js",".jsx",".ts",".tsx"],outdir:"/out",sourcemap:"inline",plugins:ie,minify:!1,bundle:!0,write:!1}),ae=Object.fromEntries(me.outputFiles.map(K=>[K.path,K.text]));let ce={css:ae==null?void 0:ae["/out/index.css"],js:ae==null?void 0:ae["/out/index.js"]};postMessage({id:p,success:!0,result:ce})}catch(X){postMessage({id:p,success:!1,error:X.message})}else O=="stop"&&ze.stop()};
