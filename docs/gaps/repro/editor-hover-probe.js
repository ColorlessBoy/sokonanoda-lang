#!/usr/bin/env node
// **编辑器 hover / 命令输出的共用真 LSP 探针**（G-102…G-104 的复现件内核）。
//
// 与 `G99-tactic-hover-name-type.js` 同一套外壳（LSP over stdio、自断言、退出码约定），
// 只是把三种形状收在一份里，按 `process.argv[2]` 选模式：
//   * `hover-shapes`  —— G-103：hover 里凡 `.sokonanoda` 语言文本都走**围栏块**
//                        （tactic 名字的类型行 / `def` 声明名的 `:=` 值块）。
//   * `command-lines` —— G-104：`#check` / `#print` 那一行显示**命令自己的输出**。
//   * `duplication`   —— G-102：**缓存/产物命中 + 编辑一次**之后，同一条命令的
//                        输出仍**恰好一条**（修前是 2 条，且随回放次数累积）。
//   * `fold`          —— G-105：`#check Eq.refl` 的**命令输出**（= 真相层
//                        `messages_at` 的文本、也就是 Infoview 显示的那一份）
//                        必须是 `… → a = a`，**不许**出现折坏的 `(α = a) a`。
//                        ⚠ 必须走**这条**路：编译路的 `ty_text` 对用户声明打的是
//                        **闭项**形状（`Eq a a`），旧口径本来就能折对 ⇒ 拿它当
//                        判据会**空转**（"咬不住的守卫等于没有"）。真现场在
//                        hover / `#check` 的**开项** pp（`Eq.{u} α a a`）。
//
// 退出码（全部 repro 脚本一致，见 docs/gaps/README.md）：
//   0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常（需要人看）
'use strict';

const { spawn } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');
const uri = (f) => 'file://' + f;
const posOf = (text, offset) => {
  const before = text.slice(0, offset);
  return {
    line: before.split('\n').length - 1,
    character: (before.split('\n').pop() || '').length,
  };
};

function startLsp() {
  const child = spawn(process.execPath, [SOKO, 'lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = Buffer.alloc(0);
  const queue = [];
  const waiters = [];
  child.stdout.on('data', (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const sep = buffer.indexOf('\r\n\r\n');
      if (sep < 0) return;
      const m = /Content-Length: (\d+)/i.exec(buffer.slice(0, sep).toString('utf8'));
      if (!m) return;
      const len = Number(m[1]);
      if (buffer.length < sep + 4 + len) return;
      const body = buffer.slice(sep + 4, sep + 4 + len).toString('utf8');
      buffer = buffer.slice(sep + 4 + len);
      const msg = JSON.parse(body);
      if (waiters.length) waiters.shift()(msg);
      else queue.push(msg);
    }
  });
  // `PROBE_TRACE=1` ⇒ 把服务端 stderr 转发出来（`SOKO_SPLICE_TRACE` / `LSP_TRACE` 等诊断）。
  child.stderr.on('data', (d) => (process.env.PROBE_TRACE ? process.stderr.write(d) : {}));
  return {
    send(msg) {
      const body = Buffer.from(JSON.stringify(msg), 'utf8');
      child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
      child.stdin.write(body);
    },
    next(timeoutMs = 120000) {
      if (queue.length) return Promise.resolve(queue.shift());
      return new Promise((resolve, reject) => {
        const t = setTimeout(() => reject(new Error('LSP timeout')), timeoutMs);
        waiters.push((m) => {
          clearTimeout(t);
          resolve(m);
        });
      });
    },
    stop() {
      try {
        child.kill();
      } catch {
        /* ignore */
      }
    },
  };
}

async function responseFor(lsp, id) {
  for (;;) {
    const m = await lsp.next();
    if (m.id === id) return m;
  }
}

/// 起一个 LSP、打开 `file`、等到第一份诊断，返回可复用的问答外壳。
async function open(lsp, dir, file, text) {
  lsp.send({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      processId: process.pid,
      rootUri: uri(dir),
      capabilities: {},
      workspaceFolders: [{ uri: uri(dir), name: 'repro' }],
    },
  });
  await responseFor(lsp, 1);
  lsp.send({ jsonrpc: '2.0', method: 'initialized', params: {} });
  lsp.send({
    jsonrpc: '2.0',
    method: 'textDocument/didOpen',
    params: { textDocument: { uri: uri(file), languageId: 'sokonanoda', version: 1, text } },
  });
  for (;;) {
    const m = await lsp.next();
    if (m.method === 'textDocument/publishDiagnostics' && m.params?.uri === uri(file)) break;
  }
}

async function main() {
  const mode = process.argv[2];
  const problems = [];
  const observed = {};
  const ok = (cond, msg) => {
    if (!cond) problems.push(msg);
  };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `editor-${mode}-`));
  fs.writeFileSync(path.join(dir, 'sokonanoda.toml'), 'entry = "Main.sokonanoda"\n');
  const fenceBlocks = (value) =>
    value
      .split('```sokonanoda\n')
      .slice(1)
      .map((rest) => rest.split('\n```')[0]);

  if (mode === 'hover-shapes' || mode === 'command-lines') {
    const src =
      'def myid : Prop -> Prop := fun (p : Prop) => p\n' +
      '\n' +
      'theorem uses_name (a : Prop) (h : a) : a := by\n' +
      '  exact h\n' +
      '\n' +
      '#check Eq.refl\n' +
      '#print myid\n';
    const file = path.join(dir, 'Main.sokonanoda');
    fs.writeFileSync(file, src);
    const lsp = startLsp();
    await open(lsp, dir, file, src);
    let id = 10;
    const hoverAt = async (offset) => {
      const myId = id++;
      lsp.send({
        jsonrpc: '2.0',
        id: myId,
        method: 'textDocument/hover',
        params: { textDocument: { uri: uri(file) }, position: posOf(src, offset) },
      });
      return (await responseFor(lsp, myId)).result ?? null;
    };
    const textOf = (h) => (h && h.contents && h.contents.value) || '';

    if (mode === 'hover-shapes') {
      // ① tactic 里的**名字**：类型行必须是**围栏块**（修前是行内码 ⇒ 不上色）。
      // 光标落在 `h` **自己**那一列（`+ 'exact '.length` = `h` 的起点；+1 会落到换行 ⇒ 不在任何名字上）。
      const onName = textOf(await hoverAt(src.indexOf('exact h') + 'exact '.length));
      observed['tactic 名字上的 hover'] = onName;
      ok(onName.length > 0, 'tactic 名字上 hover 是 null');
      ok(
        /---\n\n```sokonanoda\n/.test(onName),
        'tactic 名字的类型行不在「分割线 + 围栏块」里（修前是行内码）',
      );
      ok(!/`h : /.test(onName), 'tactic 名字的类型行还是行内码（不上色）');
      // ② `def` 声明名：类型块之后必须多一个 `:= <值>` 围栏块。
      const onDef = textOf(await hoverAt(src.indexOf('myid') + 1));
      observed['`def` 声明名上的 hover'] = onDef;
      ok(onDef.length > 0, '`def` 声明名上 hover 是 null');
      const blocks = fenceBlocks(onDef);
      ok(blocks.length >= 2, '`def` 的 hover 没有「类型块 + := 值块」两块');
      ok(
        blocks.some((b) => b.startsWith(':= ')),
        '`def` 的 hover 缺 `:= <值>` 块（类型看不出 def 的本质）',
      );
    } else {
      // ③ `#check` 那一行：必须给**表达式 : 类型**，且类型是**出口后**的
      //    （层元变量已收口成 `u`；`?u.N` 是中间态 —— 修前正是静默降级成空文本）。
      const onCheck = textOf(await hoverAt(src.indexOf('#check Eq.refl') + '#check '.length + 2));
      observed['`#check` 行上的 hover'] = onCheck;
      ok(onCheck.length > 0, '`#check` 那一行 hover 是 null');
      ok(/Eq\.refl : /.test(onCheck), '`#check` 的 hover 没有 `表达式 : 类型`');
      ok(!/\?u/.test(onCheck), '`#check` 的类型是中间态（`?u.N`）而不是出口后的文本');
      // ④ `#print` 那一行：必须给打印出来的声明（修前**一个 hover 行都没有** ⇒ null）。
      const onPrint = textOf(await hoverAt(src.indexOf('#print myid') + '#print '.length + 2));
      observed['`#print` 行上的 hover'] = onPrint;
      ok(onPrint.length > 0, '`#print` 那一行 hover 是 null（修前完全静默）');
      ok(/def myid/.test(onPrint), '`#print` 的 hover 没有打印出来的声明');
      ok(/:=/.test(onPrint), '`#print` 的 hover 缺 `:=` 值');
    }
    lsp.stop();
  } else if (mode === 'duplication') {
    // 项目模式（有 `import`）才走拼接那条路 —— 重复输出只在这里出现。
    fs.writeFileSync(
      path.join(dir, 'Lib.sokonanoda'),
      'def libid : Nat -> Nat := fun (n : Nat) => n\n',
    );
    // ⚠ **用户的稳定复现序列**（2026-10-10 12:07 用户实测：「做完 `mem_of_subset`，
    // 继续做 `eq_of_same_elements`，编辑一下，前面的 `#check` 和 `#print` 就会重复
    // 一下，现在重复 3 次了」）⇒ 夹具必须复刻**三件事**，少一件就咬不住：
    //   ① 命令（`#check`/`#print`）在**被编辑的声明之前** ⇒ 它们落进**信任前缀**，
    //      而信任前缀的报告正是**从项目产物回放**的那一份（`cmd` 归零）；
    //   ② 被编辑的声明在**长前缀**之后（用户文件 100+ 行，前面还有一整段做过的题）
    //      ⇒ 每次编辑只重算后缀、前缀照旧回放；
    //   ③ **两次编辑落在不同声明上**（先 `mem_of_subset`、再 `eq_of_same_elements`）
    //      ⇒ 每编辑一次就在回放那份上再追加一份（1 → 2 → 3）。
    const filler = (n) =>
      `theorem filler_${n} (A : Nat -> Prop) (a : Nat) : A a -> A a := by\n  intro h\n  exact h\n\n`;
    const src =
      'import Lib\n' +
      '\n' +
      '#check libid\n' +
      '#print libid\n' +
      '\n' +
      Array.from({ length: 12 }, (_, i) => filler(i + 1)).join('') +
      'theorem mem_of_subset (A B : Nat -> Prop) (h : forall (x : Nat), A x -> B x) (a : Nat) :\n' +
      '    A a -> B a := by\n' +
      '  intro ha\n' +
      '  sorry\n' +
      '\n' +
      Array.from({ length: 12 }, (_, i) => filler(i + 13)).join('') +
      // ⚠ 这里**只用 ASCII 能表达的等价说法**：语言的 `↔`/`∀` 是内建记法，
      // 写 `<->` 会 parse 失败（整份编不过 ⇒ 命令输出 0 条 ⇒ 判据空转 ✗，实测踩过）。
      'theorem eq_of_same_elements (A B : Nat -> Prop) (h : forall (x : Nat), A x -> B x) :\n' +
      '    forall (x : Nat), A x -> B x := by\n' +
      '  sorry\n';
    const file = path.join(dir, 'Main.sokonanoda');
    fs.writeFileSync(file, src);
    let stateId = 900;
    const countMessages = async (lsp, text, needle) => {
      const myId = ++stateId;
      lsp.send({
        jsonrpc: '2.0',
        id: myId,
        method: 'soko/stateAt',
        params: {
          textDocument: { uri: uri(file) },
          position: posOf(text, text.indexOf(needle) + 2),
        },
      });
      const reply = await responseFor(lsp, myId);
      // 光标落在**命令名那一列之内**（`+2` 在 `needle` 里；以前 `needle.length + 2`
      // 会落到**下一行**（甚至越过文件末尾）⇒ 数到的是别的命令、甚至 0 条 ✗）。
      const cursor = text.indexOf(needle) + 2;
      observed[`\`${needle}\` 的光标位置`] = JSON.stringify(posOf(text, cursor));
      if (reply.error) {
        problems.push(`\`${needle}\` 的 soko/stateAt 报错：${JSON.stringify(reply.error)}`);
        return -1;
      }
      const r = reply.result;
      if (!r || !Array.isArray(r.messages)) {
        observed[`\`${needle}\` 的 stateAt 响应`] = JSON.stringify(reply).slice(0, 400);
        return -1;
      }
      observed[`\`${needle}\` 的 messages`] = JSON.stringify(r.messages.map((m) => m.kind));
      return r.messages.length;
    };
    // **第一趟**：把这一版文本写进缓存 + 项目产物（下一次打开就是命中）。
    {
      const lsp = startLsp();
      await open(lsp, dir, file, src);
      lsp.stop();
    }
    // **第二趟**：命中产物打开 ⇒ 按用户的顺序编辑 ⇒ 每次编辑后数条数。
    const lsp = startLsp();
    await open(lsp, dir, file, src);
    const steps = [
      // ① 做 `mem_of_subset`（把 `sorry` 换成证明）
      {
        why: '做 mem_of_subset（第一个声明）',
        edit: (t) =>
          t.replace(
            'theorem mem_of_subset (A B : Nat -> Prop) (h : forall (x : Nat), A x -> B x) (a : Nat) :\n    A a -> B a := by\n  intro ha\n  sorry\n',
            'theorem mem_of_subset (A B : Nat -> Prop) (h : forall (x : Nat), A x -> B x) (a : Nat) :\n    A a -> B a := by\n  intro ha\n  exact h a ha\n',
          ),
      },
      // ②③④ 做 `eq_of_same_elements`（后面的声明）——用户说「编辑一下，就重复一下」。
      // ⚠ 编辑必须是**能编过的改动**（这里插注释）：夹具一旦编不过，`#check`/`#print`
      // 那两行根本不会产出命令输出 ⇒ 读数是 0 条，判据空转 ✗（踩过）。
      {
        why: '做 eq_of_same_elements（1）',
        edit: (t) => t.replace('    forall (x : Nat), A x -> B x := by\n  sorry\n', '    forall (x : Nat), A x -> B x := by\n  -- step 1\n  sorry\n'),
      },
      {
        why: '做 eq_of_same_elements（2）',
        edit: (t) => t.replace('  -- step 1\n  sorry\n', '  -- step 1\n  -- step 2\n  sorry\n'),
      },
      {
        why: '做 eq_of_same_elements（3）',
        edit: (t) => t.replace('  -- step 2\n  sorry\n', '  -- step 2\n  -- step 3\n  sorry\n'),
      },
    ];
    let text = src;
    let version = 1;
    let worst = 0;
    const perRound = [];
    for (const [i, step] of steps.entries()) {
      const next = step.edit(text);
      if (next === text) {
        problems.push(`夹具自检：第 ${i + 1} 步（${step.why}）没有改动文本 ⇒ 判据空转 ✗`);
        break;
      }
      text = next;
      version += 1;
      lsp.send({
        jsonrpc: '2.0',
        method: 'textDocument/didChange',
        params: {
          textDocument: { uri: uri(file), version },
          contentChanges: [{ text }],
        },
      });
      for (;;) {
        const m = await lsp.next();
        if (m.method === 'textDocument/publishDiagnostics' && m.params?.uri === uri(file)) break;
      }
      const checks = await countMessages(lsp, text, '#check libid');
      const prints = await countMessages(lsp, text, '#print libid');
      perRound.push({ step: step.why, checks, prints });
      worst = Math.max(worst, checks, prints);
    }
    lsp.stop();
    observed['每步份数（用户序列）'] = JSON.stringify(perRound);
    for (const r of perRound) {
      ok(
        r.checks === 1 && r.prints === 1,
        `${r.step} 之后：#check=${r.checks} 条 · #print=${r.prints} 条（应为 1/1 —— 用户现场会累积到 3）`,
      );
    }
    ok(worst <= 1, `「每编辑一次 +1」仍然存在（最大 ${worst} 份）`);
  } else if (mode === 'fold') {
    // **G-105**：真现场 = `#check` 的**命令输出**（开项 pp `Eq.{u} α a a`）。
    const src = 'def myid : Prop -> Prop := fun (p : Prop) => p\n\n#check Eq.refl\n';
    const file = path.join(dir, 'Main.sokonanoda');
    fs.writeFileSync(file, src);
    const lsp = startLsp();
    await open(lsp, dir, file, src);
    const myId = 777;
    lsp.send({
      jsonrpc: '2.0',
      id: myId,
      method: 'soko/stateAt',
      params: {
        textDocument: { uri: uri(file) },
        position: posOf(src, src.indexOf('#check Eq.refl') + 2),
      },
    });
    const reply = await responseFor(lsp, myId);
    lsp.stop();
    const messages = (reply.result && reply.result.messages) || [];
    observed['`#check Eq.refl` 的 messages'] = JSON.stringify(
      messages.map((m) => m.text),
    );
    ok(messages.length === 1, `\`#check Eq.refl\` 的输出应有 1 条，实得 ${messages.length}`);
    const text = messages.length ? String(messages[0].text) : '';
    ok(/Eq\.refl : /.test(text), `命令输出缺 \`表达式 : 类型\`：${text}`);
    ok(!/\(α = /.test(text), `类型被折坏（部分应用当完全应用）：${text}`);
    ok(/a = a/.test(text), `类型应折成 \`a = a\`：${text}`);
  } else {
    console.error(`未知模式：${mode}（可用：hover-shapes / command-lines / duplication / fold）`);
    process.exit(2);
  }

  if (problems.length === 0) {
    console.log(`${mode} 已修 ✓ —— 断言全绿`);
    process.exit(1);
  }
  console.log(`${mode} 缺口仍在（或出现回归）✗：`);
  for (const p of problems) console.log('  - ' + p);
  for (const [label, value] of Object.entries(observed)) {
    console.log(`--- ${label} ---\n${String(value).slice(0, 600)}`);
  }
  process.exit(0);
}

main().catch((e) => {
  console.error('探针异常（按环境问题处理，不是「已修」）：', e && e.message ? e.message : e);
  process.exit(2);
});
