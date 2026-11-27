# 45–60s demo script — screen recording / Bilibili / YouTube

The 7-second loop in `site/assets/demo-vscode.gif` already shows the core moment
(goal → three tactics → checked). This script expands it into a narrated clip. It
uses only real UI: no mockups, no fake typing.

## Shot list

| Time | Screen | Action |
|---|---|---|
| 0:00–0:05 | Desktop | Open VS Code → Extensions → search `sokonanoda` → Install. Do not speed this up; the point is that it is one install. |
| 0:05–0:11 | Editor | Open `courses/set-theory/units/I.1/unit02-subsets-empty.sokonanoda`; put the cursor on the `sorry` under `subset_trans`. The right panel shows the goal. |
| 0:11–0:19 | Editor | Type `intro x hx`; pause on the new goal (`⊢ x ∈ C`). |
| 0:19–0:26 | Editor | Type `apply h2`; the goal becomes `⊢ x ∈ B`. |
| 0:26–0:31 | Editor | Type `exact h1 x hx`; the panel shows `🎉 已无目标 ✓` and the declaration is checked. |
| 0:31–0:38 | Editor | Hover a name to show its type; move the cursor to show the goal follows the caret. |
| 0:38–0:48 | End card | Site URL, GitHub, "three courses · 2,022 kernel-checked exercises · no Lean/Rust install". |

## English narration

> Formal proofs usually start with installing a toolchain. This one starts with a
> proof.
>
> Install the Sokonanoda extension. That is the whole setup — the language server
> and the command-line checker ship inside it, so there is no Lean and no Rust to
> install.
>
> Open a lesson and put the cursor on a `sorry`. The panel on the right is the
> goal, computed by the kernel. Type `intro x hx` — the goal updates. `apply h2`.
> `exact h1 x hx`. The panel says there are no goals left, and the declaration is
> checked by the kernel — not by text matching.
>
> Three courses, 2,022 exercises, every one checked by the same kernel. Free and
> open source, offline after install.
>
> Link below.

## 中文旁白

> 学形式化证明，通常先装一整套工具链。这个不用——打开就能写。
>
> 在 VS Code 里装 Sokonanoda 插件，安装就结束了：语言服务器和命令行判卷器都在插件里，
> 不用装 Lean，也不用装 Rust。
>
> 打开一课，把光标停在 `sorry` 上。右边是内核实时算出来的目标。输入 `intro x hx`，
> 目标变了；`apply h2`；`exact h1 x hx`。面板显示已无目标，声明由内核判定通过——
> 不是文本匹配。
>
> 三门课，2,022 道练习，全部由同一个内核判卷。开源免费，装完离线可用。
>
> 链接在下面。

## 15-second silent cut (Product Hunt / social)

Use the existing GIF as the visual; burn in these captions:

1. `Install the extension. That's the setup.`
2. `Cursor on sorry → kernel-computed goal.`
3. `Three tactics → checked by the kernel.`
4. `No Lean. No Rust. Works offline.`

Chinese version:

1. `装个插件，就能开始。`
2. `光标停在 sorry，内核算出目标。`
3. `写完，由内核判卷。`
4. `不用 Lean，不用 Rust，离线可用。`

## Recording notes

- Record at 1080p or higher, 30 fps; keep the editor font at the size used in the
  screenshots so the goal panel is readable on a phone.
- Do not add a fake progress bar, fake terminal output, or a mockup of the
  Marketplace page. If a shot needs the Marketplace, record the real page.
- Keep the mouse still while typing; jump cuts between tactics are fine, but do
  not speed up the kernel's response.
- End on the site URL for at least 4 seconds.
- The 7-second GIF is the fallback for platforms that reject video; it is already
  in `assets/demo-vscode.gif` (copy of `site/assets/demo-vscode.gif`).
