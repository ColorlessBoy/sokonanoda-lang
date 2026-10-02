const { execFileSync } = require('child_process'); const fs = require('fs');
const BIN = process.env.SOKO_BIN || './target/debug/sokonanoda';
const ENV = Object.assign({}, process.env, { SOKONANODA_NO_CACHE: '1' });
const files = execFileSync('bash', ['-c', 'ls courses/set-theory/units/*/*.sokonanoda'], {encoding:'utf8'}).trim().split('\n');
const run = a => { try { return execFileSync(BIN, a, {encoding:'utf8', maxBuffer: 1<<28, env: ENV}); } catch(e){ return e.stdout||''; } };
function firstTactic(lines, i0) {
  for (let i=i0; i<Math.min(i0+12, lines.length); i++) {
    const m = lines[i].indexOf(':= by'); if (m < 0) continue;
    const rest = lines[i].slice(m+5);
    if (rest.trim()) return [i+1, m+6+(rest.length-rest.trimStart().length)];
    for (let j=i+1; j<lines.length; j++) {
      if (!lines[j].trim() || lines[j].trim().startsWith('--')) continue;
      return [j+1, (lines[j].length-lines[j].trimStart().length)+1];
    }
  }
  return null;
}
let total=0, bad=0, same=0; const byFile={};
for (const f of files) {
  let j; try { j = JSON.parse(run(['query','goals','--file',f,'--compact'])); } catch(e) { continue; }
  if (!Array.isArray(j.data)) continue;
  const src = fs.readFileSync(f,'utf8').split('\n');
  for (const d of j.data) {
    if (!d || !d.name || (d.status && d.status!=='open')) continue;
    let idx=-1;
    for (let i=0;i<src.length;i++) if (new RegExp('^\\s*(theorem|lemma)\\s+'+d.name.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')+'\\b').test(src[i])) { idx=i; break; }
    if (idx<0) continue;
    const tl = firstTactic(src, idx); if (!tl) continue;
    let s; try { s = JSON.parse(run(['query','state','--file',f,'--line',String(tl[0]),'--col',String(tl[1]),'--compact'])); } catch(e) { continue; }
    const g = Array.isArray(s.data) ? s.data[0] : s.data; if (!g) continue;
    total++;
    const eq = JSON.stringify((g.binders||[]).map(x=>x.name)) === JSON.stringify((d.binders||[]).map(x=>x.name))
            && (g.goal||'') === (d.goal||'');
    if (eq) same++; else { bad++; const k=f.replace('courses/set-theory/units/',''); byFile[k]=(byFile[k]||0)+1; }
  }
}
console.log(`开放声明 ${total}｜顶≡底 ${same}｜顶≠底 ${bad}`);
for (const k of Object.keys(byFile).sort((a,b)=>byFile[b]-byFile[a]).slice(0,20)) console.log(`  ${byFile[k]}  ${k}`);
