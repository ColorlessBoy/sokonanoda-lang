#!/usr/bin/env node
// G-38 自断言复现：**线 C 的记法折叠只认 infix 族** ⇒ 声明栏里 `forall` 不折成 `∀`。
//
// 用户原话（2026-09-23）：「infoview里的"声明"栏，"forall" 可以用 "∀"，对应背后
// 是不是丢了一批符号的改写呢？查一下完整的bug产生的原因，统一一起修掉；」
//
// **机制（读码 + 实测）**：`crates/front/src/display.rs::fold_spine` 明写
// 「第一刀只做二元 infix 族」并 `if !matches!(decl.assoc, Infix|Infixl|Infixr)
// { return None; }` ⇒ **prefix（`𝒫`）、postfix（`ᶜ`）、binder（`∀`/`∃`）、
// 零元（`∅`）全都漏折**。实测同一份夹具里：`∈` 折了、`forall` 没折。
//
// 退出码：0 = 缺口仍在 · 1 = 已修 · 2 = 环境/形状异常。
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SOKO = path.join(ROOT, 'scripts', 'soko');

const SRC = `def Set (α : Type) : Type := α -> Prop
def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a
infix:50 " ∈ " => Set.mem
def Set.powerset (α : Type) (A : Set α) : Set α := A
prefix:100 " 𝒫 " => Set.powerset
def Set.compl (α : Type) (A : Set α) : Set α := A
postfix:100 " ᶜ " => Set.compl
def Set.image (α : Type) (f : α -> α) (A : Set α) : Set α := A
infixr:80 " '' " => Set.image

theorem t_infix (α : Type) (a : α) (A : Set α) : a ∈ A -> a ∈ A := fun h => h
`;

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'g38-'));
const file = path.join(dir, 'Kinds.sokonanoda');
fs.writeFileSync(file, SRC);
const out = spawnSync(process.execPath, [SOKO, 'query', 'goals', '--file', file], {
  encoding: 'utf8',
  cwd: ROOT,
});
fs.rmSync(dir, { recursive: true, force: true });

let goals;
try {
  goals = JSON.parse(out.stdout).data;
} catch {
  console.error('复现脚本自身出错（形状异常）：', out.stdout.slice(0, 200), out.stderr.slice(0, 200));
  process.exit(2);
}
const tyOf = (name) => (goals.find((g) => g.name === name) || {}).ty || '';
const infix = tyOf('t_infix');
console.log(`t_infix ty = ${infix}`);

// **夹具前提**（两条都要成立，否则量到的不是折叠覆盖——G-36 踩过"因为错的原因
// 为真"的坑）：① 这条声明确实 elaborate 了（`ty` 非空）；② infix 确实折了。
if (!infix) {
  console.error('复现脚本自身出错：`ty` 是空的 ⇒ 夹具没 elaborate，量不到东西。');
  process.exit(2);
}
if (!infix.includes('∈')) {
  console.error('复现脚本自身出错：连 infix 都没折 ⇒ 形状不对。');
  process.exit(2);
}
if (infix.includes('∀')) {
  console.log('结论：G-38 已修——telescope 也打成 `∀` 了。');
  process.exit(1);
}
console.error('结论：G-38 仍在——声明栏的 `ty` 里 `∈` 折了、`forall` 没折。');
console.error('      机制比"过滤器"深一层：① `fold_spine` 只放行 infix 族；');
console.error('      ② `∀`/`∃` **不在内建记法表**（表里只有 `∧ ∨ ↔ ¬ = ≠`）；');
console.error('      ③ 这里的 `forall` 是**内核打的 telescope**（不是源码里的 `∀`）');
console.error('      ⇒ 要修得同时补表 + 认 `forall (x : T), body` 这个形状。');
process.exit(0);
