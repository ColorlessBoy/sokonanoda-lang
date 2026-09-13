#!/usr/bin/env python3
"""生成官网的 GIF/图片演示（site/assets/demos/）。

设计与口径见 docs/design/site.md §7（首页 v2）。三个演示对应产品主循环的
三个卖点：输入期补全（弹窗从第一个字符起就在）、组合关键字、内核判卷。

可复现：纯 PIL 逐帧绘制，无外部素材；重跑幂等。运行需要 Pillow（仅维护者；
学习者路径零工具链的纪律不受影响——产物已提交入库）。

用法：python3 scripts/gen-site-demos.py
"""

from __future__ import annotations

import math
import hashlib
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "site" / "assets" / "demos"

# ── VS Code Dark+ 配色 ─────────────────────────────────────────────
EDITOR_BG = (30, 30, 30)
SIDEBAR_BG = (37, 37, 38)
ACTIVITY_BG = (51, 51, 51)
TITLE_BG = (60, 60, 60)
TABBAR_BG = (37, 37, 38)
STATUS_BG = (0, 120, 170)
POPUP_BG = (37, 37, 38)
POPUP_BORDER = (69, 69, 69)
SELECT_BG = (4, 57, 94)
FG = (212, 212, 212)
LINE_NO = (133, 133, 133)
KEYWORD = (197, 134, 192)  # fun/intro/by
TYPE = (78, 201, 176)      # Prop/And
IDENT = (156, 220, 254)
PUNCT = (212, 212, 212)
COMMENT = (106, 153, 85)
ERROR = (241, 76, 76)
CURSOR = (174, 174, 174)
OK_GREEN = (135, 214, 132)

CODE_FONT = "/System/Library/Fonts/Menlo.ttc"
CJK_FONT = "/System/Library/Fonts/Hiragino Sans GB.ttc"

LH = 22          # 行高
PAD_L = 16       # 编辑器左内边距
GUTTER = 56      # 行号槽宽

FS_CODE = 15
FS_UI = 12


FONTS_AVAILABLE = all(
    Path(p).exists() for p in (CODE_FONT, CJK_FONT)
)


def font(path: str, size: int, index: int = 0):
    return ImageFont.truetype(path, size, index=index)


F_CODE = font(CODE_FONT, FS_CODE)
F_CODE_B = font(CODE_FONT, FS_CODE)  # Menlo 无粗体档，沿用
F_UI = font(CJK_FONT, FS_UI)
F_SMALL = font(CJK_FONT, 11)
F_LINENO = font(CODE_FONT, FS_CODE - 2)


def text_w(draw, s, f):
    return draw.textlength(s, font=f)


# ── 语法着色：把一行源码拆成 (text, color) 片段 ────────────────────
KEYWORDS = {"fun", "funintro", "by", "intro", "exact", "apply", "sorry", "assumption"}


def tokenize(line: str) -> list[tuple[str, tuple]]:
    toks: list[tuple[str, tuple]] = []
    i = 0
    buf = ""
    while i < len(line):
        ch = line[i]
        if ch.isalnum() or ch in "_.":
            buf += ch
            i += 1
            continue
        if buf:
            toks.append(buf)
            buf = ""
        toks.append(ch)
        i += 1
    if buf:
        toks.append(buf)
    out = []
    for t in toks:
        head, _, _ = t.partition(".")
        if head in KEYWORDS or t in KEYWORDS:
            out.append((t, KEYWORD))
        elif head in ("Prop", "And", "Or", "Not", "True", "False", "Nat", "Eq"):
            out.append((t, TYPE))
        elif t[0].isupper():
            out.append((t, IDENT))
        else:
            out.append((t, PUNCT))
    return out


# ── 界面骨架 ───────────────────────────────────────────────────────
W, H = 1180, 700
EDITOR_X = 48 + 190  # activity + sidebar


def chrome(draw: ImageDraw.ImageDraw, filename: str, status: str, status_warn: bool = False):
    # 标题栏
    draw.rectangle([0, 0, W, 34], fill=TITLE_BG)
    for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
        draw.ellipse([14 + i * 22, 12, 26 + i * 22, 24], fill=c)
    f = font(CODE_FONT, 12)
    tw = text_w(draw, filename, f)
    draw.text(((W - tw) / 2, 10), filename, fill=(180, 180, 180), font=f)
    # 活动栏
    draw.rectangle([0, 34, 48, H - 26], fill=ACTIVITY_BG)
    for i in range(4):
        y = 52 + i * 44
        color = (255, 255, 255) if i == 0 else (120, 120, 120)
        draw.rectangle([16, y, 32, y + 22], outline=color, width=2)
    # 侧栏
    draw.rectangle([48, 34, 48 + 190, H - 26], fill=SIDEBAR_BG)
    draw.text((62, 48), "资源管理器", fill=(200, 200, 200), font=F_UI)
    draw.rectangle([58, 84, 224, 108], fill=(58, 58, 58))
    draw.text((70, 88), "PLAYGROUND", fill=(230, 230, 230), font=F_SMALL)
    for i, name in enumerate(["playground.sokonanoda", "course/", "examples/", "skills/"]):
        y = 116 + i * 26
        if i == 0:
            draw.rectangle([58, y - 2, 224, y + 22], fill=(47, 61, 76))
        draw.text((78, y), name, fill=(225, 225, 225) if i == 0 else (170, 170, 170), font=F_SMALL)
    # 标签栏
    draw.rectangle([EDITOR_X, 34, W, 66], fill=TABBAR_BG)
    draw.rectangle([EDITOR_X, 34, EDITOR_X + 250, 66], fill=EDITOR_BG)
    draw.text((EDITOR_X + 14, 42), filename, fill=(255, 255, 255), font=F_SMALL)
    draw.ellipse([EDITOR_X + 232, 46, EDITOR_X + 240, 54], fill=(230, 230, 230))
    # 状态栏
    sb = (170, 110, 20) if status_warn else STATUS_BG
    draw.rectangle([0, H - 26, W, H], fill=sb)
    draw.text((14, H - 21), status, fill=(255, 255, 255), font=F_SMALL)
    draw.text((W - 250, H - 21), "sokonanoda-lsp  ✓ 内核判卷", fill=(255, 255, 255), font=F_SMALL)


def editor_frame(
    lines: list[list[tuple[str, tuple]]],
    cursor: tuple[int, int] | None = None,
    popup: dict | None = None,
    hover: dict | None = None,
    squiggle: tuple[int, int, int, int] | None = None,
    inlays: dict[int, str] | None = None,
    status: str = "",
    status_warn: bool = False,
    filename: str = "playground.sokonanoda",
) -> Image.Image:
    img = Image.new("RGB", (W, H), EDITOR_BG)
    draw = ImageDraw.Draw(img)
    chrome(draw, filename, status, status_warn)
    top = 78
    # 行号 + 代码
    for li, line in enumerate(lines):
        y = top + li * LH
        draw.text((16, y + 2), f"{li + 1}", fill=LINE_NO, font=F_LINENO)
        x = EDITOR_X + PAD_L
        for tok, color in line:
            draw.text((x, y), tok, fill=color, font=F_CODE)
            x += text_w(draw, tok, F_CODE)
    # 波浪线（诊断）
    if squiggle:
        x0, y0, x1, li = squiggle
        y = top + li * LH + FS_CODE + 4
        pts = []
        n = int((x1 - x0) / 6)
        for k in range(n + 1):
            pts.append((x0 + k * 6, y + (2 if k % 2 == 0 else 0)))
        draw.line(pts, fill=ERROR, width=1)
    # 行内提示（inlay / 内核结论）
    if inlays:
        for li, label in inlays.items():
            y = top + li * LH + 2
            line = lines[li]
            x = EDITOR_X + PAD_L + text_w(draw, "".join(t for t, _ in line), F_CODE) + 12
            tw = text_w(draw, label, F_SMALL)
            draw.rectangle([x, y - 1, x + tw + 10, y + FS_CODE + 1], fill=(51, 51, 51))
            draw.text((x + 5, y + 1), label, fill=(150, 170, 200) if ": " in label else OK_GREEN, font=F_SMALL)
    # 光标
    if cursor:
        li, col = cursor
        y = top + li * LH
        x = EDITOR_X + PAD_L + col
        draw.rectangle([x, y, x + 2, y + FS_CODE + 4], fill=CURSOR)
    # 补全弹窗
    if popup:
        _draw_popup(draw, popup, cursor)
    # hover 组件
    if hover:
        _draw_hover(draw, hover, cursor)
    return img


def _draw_popup(draw, popup, cursor):
    items = popup["items"]
    sel = popup.get("selected", 0)
    li, col = cursor
    x = EDITOR_X + PAD_L + col + 8
    y = 78 + li * LH + LH + 4
    w = max(text_w(draw, it["label"], F_SMALL) + text_w(draw, it.get("detail", ""), F_SMALL) + 56 for it in items) + 20
    h = len(items) * 24 + 8
    draw.rectangle([x, y, x + w, y + h], fill=POPUP_BG, outline=POPUP_BORDER, width=1)
    draw.line([x, y, x, y + h], fill=(0, 122, 204), width=2)
    for i, it in enumerate(items):
        ry = y + 4 + i * 24
        if i == sel:
            draw.rectangle([x + 2, ry - 1, x + w - 2, ry + 21], fill=SELECT_BG)
        # kind 图标
        draw.rectangle([x + 10, ry + 5, x + 22, ry + 17], outline=(200, 160, 220), width=1)
        draw.text((x + 28, ry + 1), it["label"], fill=(230, 230, 230), font=F_SMALL)
        if it.get("detail"):
            dw = text_w(draw, it["label"], F_SMALL)
            draw.text((x + 34 + dw, ry + 1), it["detail"], fill=(150, 150, 150), font=F_SMALL)
        if it.get("selected_badge"):
            bw = text_w(draw, it["selected_badge"], F_SMALL)
            draw.text((x + w - bw - 12, ry + 1), it["selected_badge"], fill=(120, 190, 255), font=F_SMALL)


def _draw_hover(draw, hover, cursor):
    li, col = cursor
    x = EDITOR_X + PAD_L + col + 10
    y = 78 + li * LH + LH + 6
    w = hover["w"]
    h = hover["h"]
    draw.rectangle([x, y, x + w, y + h], fill=(37, 37, 38), outline=(69, 69, 69), width=1)
    yy = y + 10
    for line, f, color in hover["lines"]:
        draw.text((x + 12, yy), line, fill=color, font=f)
        yy += f.size + 7
    if hover.get("button"):
        bw = hover["button_w"]
        draw.rectangle([x + 12, yy + 2, x + 12 + bw, yy + 26], fill=(0, 90, 158))
        bt = hover["button"]
        tw = text_w(draw, bt, F_UI)
        draw.text((x + 12 + (bw - tw) / 2, yy + 6), bt, fill=(255, 255, 255), font=F_UI)


# ── 代码行构建 ─────────────────────────────────────────────────────
def L(*parts: tuple[str, tuple] | str) -> list[tuple[str, tuple]]:
    out = []
    for p in parts:
        out.extend((t, c) for t, c in (tokenize(p) if isinstance(p, str) else [p]))
    return out


def line_width(draw, line):
    return text_w(draw, "".join(t for t, _ in line), F_CODE)


# ── 演示 1：输入期补全 + Tab 接受展开 ──────────────────────────────
BASE_LINES = [
    L("axiom And : Prop -> Prop -> Prop"),
    L("axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b"),
    L(""),
    L("theorem and_swap : (a : Prop) -> (b : Prop) -> And a b -> And b a :="),
    L("  fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => "),
]


def demo_completion() -> list[tuple[Image.Image, int]]:
    """值位 `funintro`：逐键输入 → 弹窗从第一个字符起就在 → Tab 补全单词 →
    骨架态出现 → 接受 → `sorry` 处于选中态（下一次输入直接覆盖）。"""
    frames = []
    probe = ImageDraw.Draw(Image.new("RGB", (8, 8)))
    head = "theorem t : Q -> P := "
    base = [
        L("axiom P : Prop"),
        L("axiom Q : Prop"),
        L("axiom proofP : P"),
        L(""),
        L(head + "sorry"),
    ]
    li = 4
    caret_px = int(text_w(probe, head, F_CODE)) - 24
    # ① 逐键敲 funintro：每个前缀都有键入态项（preselect）
    typed = ""
    for ch in "funintro":
        typed += ch
        lines = [l[:] for l in base]
        lines[4] = L(head + typed)
        cur = caret_px + int(text_w(probe, typed, F_CODE))
        popup = {
            "items": [
                {"label": "funintro（替换源代码）", "selected_badge": "Tab 接受"},
                {"label": "fun"},
            ],
            "selected": 0,
        }
        frames.append(
            (editor_frame(lines, cursor=(li, cur), popup=popup,
                          status="练习待填：t", status_warn=True),
             260 if len(typed) == 1 else 130)
        )
    # ② 整词 + 尾随空格：仍是键入态项
    lines = [l[:] for l in base]
    lines[4] = L(head + "funintro ")
    frames.append(
        (editor_frame(lines, cursor=(li, caret_px + int(text_w(probe, "funintro ", F_CODE))),
                      popup={"items": [
                          {"label": "funintro（替换源代码）", "selected_badge": "Tab 接受"},
                      ], "selected": 0},
                      status="练习待填：t", status_warn=True),
         400)
    )
    # ③ 骨架态：接受补全 → `fun (x : Q) => ${0:sorry}`，sorry 落盘并选中
    lines = [l[:] for l in base]
    lines[4] = L(head + "fun (x : Q) => sorry")
    sorry_px = caret_px + int(text_w(probe, "fun (x : Q) => ", F_CODE))
    img = editor_frame(lines, status="练习待填：t（sorry 已选中，输入即覆盖）", status_warn=True)
    draw = ImageDraw.Draw(img)
    sy = 78 + li * LH
    draw.rectangle([EDITOR_X + PAD_L + sorry_px, sy - 1,
                    EDITOR_X + PAD_L + sorry_px + text_w(probe, "sorry", F_CODE), sy + FS_CODE + 3],
                   outline=(120, 190, 255), width=2)
    frames.append((img, 1100))
    return frames


# ── 演示 2：组合关键字 + 内核判定 ─────────────────────────────────
# ── 静态图 1：hover 按钮 ──────────────────────────────────────────
def demo_hover_png() -> Image.Image:
    lines = [
        L("axiom P : Prop"),
        L("axiom Q : Prop"),
        L("axiom proofP : P"),
        L(""),
        L("theorem t : Q -> P := funintro"),
    ]
    probe = ImageDraw.Draw(Image.new("RGB", (8, 8)))
    prefix = "theorem t : Q -> P := "
    cursor_col = int(text_w(probe, prefix, F_CODE)) - 24
    img = editor_frame(
        [l[:] for l in lines],
        cursor=(4, cursor_col + int(text_w(probe, "funintro", F_CODE))),
        status="练习待填：t",
        status_warn=True,
    )
    draw = ImageDraw.Draw(img)
    hover = {
        "w": 430,
        "h": 150,
        "button": "替换源代码 funintro",
        "button_w": 190,
        "lines": [
            ("值位 funintro：一次引入剩余的全部 binder。", F_UI, (230, 230, 230)),
            ("", F_SMALL, FG),
            ("不替换也完全等价——funintro 本身就是一次合法", F_SMALL, (200, 200, 200)),
            ("作答；它等价于：", F_SMALL, (200, 200, 200)),
            ("fun (x : Q) => sorry", F_CODE, TYPE),
        ],
    }
    _draw_hover(draw, hover, (4, cursor_col + int(text_w(probe, "funintro", F_CODE))))
    return img


# ── 静态图 2：内核判卷 ────────────────────────────────────────────
def demo_kernel_png() -> Image.Image:
    lines = [
        L("axiom And : Prop -> Prop -> Prop"),
        L("axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b"),
        L(""),
        L("theorem wrong : (a : Prop) -> (b : Prop) -> And a b -> And b a :="),
        L("  fun (a : Prop) => fun (b : Prop) => fun (x : And a b) => And.intro a b"),
    ]
    probe = ImageDraw.Draw(Image.new("RGB", (8, 8)))
    wline = lines[4]
    x0 = EDITOR_X + PAD_L + int(text_w(probe, "".join(t for t, _ in wline)[:34], F_CODE))
    img = editor_frame(
        [l[:] for l in lines],
        squiggle=(x0, 0, x0 + 130, 4),
        status="kernel-rejected：内核拒绝",
        status_warn=True,
    )
    draw = ImageDraw.Draw(img)
    # 诊断悬浮框
    hx, hy = EDITOR_X + PAD_L + 60, 78 + 4 * LH + LH + 10
    draw.rectangle([hx, hy, hx + 470, hy + 108], fill=(37, 37, 38), outline=(69, 69, 69), width=1)
    draw.line([hx, hy, hx, hy + 108], fill=ERROR, width=2)
    draw.text((hx + 12, hy + 10), "kernel-rejected  ✗", fill=ERROR, font=F_UI)
    draw.text((hx + 12, hy + 36), "内核拒绝了这个证明：And.intro a b 的结论是", font=F_SMALL, fill=(210, 210, 210)) if False else None
    draw.text((hx + 12, hy + 34), "内核拒绝了这个证明。检查提示：", fill=(210, 210, 210), font=F_SMALL)
    draw.text((hx + 12, hy + 58), "And.intro b a 的两个前提依次是 b、a——取用的", fill=(200, 200, 200), font=F_SMALL)
    draw.text((hx + 12, hy + 80), "顺序写反了。判对判错由内核说了算。", fill=(200, 200, 200), font=F_SMALL)
    return img


# ── GIF 组装 ──────────────────────────────────────────────────────
def save_gif(frames: list[tuple[Image.Image, int]], path: Path) -> None:
    ims = [f for f, _ in frames]
    durs = [d for _, d in frames]
    # 用首帧的调色板量化（暗色 UI，颜色少，画质稳定）
    pal = ims[0].convert("P", palette=Image.ADAPTIVE, colors=64)
    pal.save(
        path,
        save_all=True,
        append_images=[im.convert("P", palette=Image.ADAPTIVE, colors=64) for im in ims[1:]],
        duration=durs,
        loop=0,
        optimize=True,
    )


def render_all() -> dict[str, list[tuple[Image.Image, int]]]:
    """全部演示的帧序列（单一事实源：页面与 --check 都从这里出发）。"""
    return {
        "demo-completion.gif": demo_completion(),
        "demo-hover.png": [(demo_hover_png(), 0)],
        "demo-kernel.png": [(demo_kernel_png(), 0)],
    }


def _quantized_rgb(img: Image.Image) -> Image.Image:
    # 与 save_gif 的调色板量化同口径：GIF 解码出来的像素 = 量化后的像素。
    return img.convert("P", palette=Image.ADAPTIVE, colors=64).convert("RGB")


def frame_fingerprint(frames: list[tuple[Image.Image, int]]) -> list[str]:
    return [
        hashlib.sha256(_quantized_rgb(f).tobytes()).hexdigest()
        for f, _ in frames
    ]


def frame_fingerprint_raw(frames: list[tuple[Image.Image, int]]) -> list[str]:
    return [hashlib.sha256(f.convert("RGB").tobytes()).hexdigest() for f, _ in frames]


def main() -> None:
    import sys

    check = "--check" in sys.argv
    if check and not FONTS_AVAILABLE:
        # 渲染依赖 macOS 系统字体（Menlo + Hiragino）。字体不可用的平台
        # （CI Linux runner）跳过校验——例行校验在维护者本机强制执行；
        # 渲染本身不进 CI（这也是演示不经 CI 重生成的原因）。
        print("demos check skipped: platform fonts unavailable", file=sys.stderr)
        return
    all_frames = render_all()
    if check:
        # 例行化校验（CI / pages 用）：仓库里的演示若与当前渲染不一致
        # （交互或文案改了但没重跑本脚本），报错并给出修复命令。
        bad = []
        for name, frames in all_frames.items():
            path = OUT / name
            if not path.exists():
                bad.append(f"{name}: missing — run `python3 scripts/gen-site-demos.py`")
                continue
            existing = Image.open(path)
            existing_frames = []
            i = 0
            while True:
                existing.seek(i)
                existing_frames.append(existing.convert("RGB"))
                i += 1
                try:
                    existing.seek(i)
                except EOFError:
                    break
            if name.endswith(".gif"):
                want = frame_fingerprint(frames)  # GIF：量化后的像素
            else:
                want = frame_fingerprint_raw(frames)  # PNG：原始像素
            got = [hashlib.sha256(im.tobytes()).hexdigest() for im in existing_frames]
            if want != got:
                bad.append(f"{name}: stale — run `python3 scripts/gen-site-demos.py`")
        if bad:
            for b in bad:
                print(f"DEMO STALE: {b}", file=sys.stderr)
            sys.exit(1)
        print(f"demos up-to-date ({len(all_frames)} files)")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    for name, frames in all_frames.items():
        if name.endswith(".gif"):
            save_gif(frames, OUT / name)
        else:
            frames[0][0].save(OUT / name, optimize=True)
    for f in sorted(OUT.iterdir()):
        print(f"  {f.name}  {f.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
