#!/usr/bin/env python3
"""生成官网的 GIF/图片演示（site/assets/demos/）。

设计与口径见 docs/design/site.md §7（首页 v2）。四个演示对应产品主循环的
四个典型瞬间：hover `sorry` 看洞的期望类型与剩余目标、内核判卷、输入期
补全（弹窗从第一个字符起就在）、`--json` 事件流（任意 code agent 判卷）。

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
TYPE = (78, 201, 176)  # Prop/And
IDENT = (156, 220, 254)
PUNCT = (212, 212, 212)
COMMENT = (106, 153, 85)
ERROR = (241, 76, 76)
CURSOR = (174, 174, 174)
OK_GREEN = (135, 214, 132)

CODE_FONT = "/System/Library/Fonts/Menlo.ttc"
CJK_FONT = "/System/Library/Fonts/Hiragino Sans GB.ttc"

LH = 22  # 行高
PAD_L = 16  # 编辑器左内边距
GUTTER = 56  # 行号槽宽

FS_CODE = 15
FS_UI = 12


FONTS_AVAILABLE = all(Path(p).exists() for p in (CODE_FONT, CJK_FONT))


def font(path: str, size: int, index: int = 0):
    return ImageFont.truetype(path, size, index=index)


# 字体惰性初始化：--check 在缺字体的平台（CI Linux）不渲染，也不该在
# import 时就触碰系统字体路径。
_FONTS: dict = {}

_FONT_SPECS = {
    "code": (CODE_FONT, FS_CODE),
    "ui": (CJK_FONT, FS_UI),
    "small": (CJK_FONT, 11),
    "lineno": (CODE_FONT, FS_CODE - 2),
}


def _F(key: str):
    if key not in _FONTS:
        path, size = _FONT_SPECS[key]
        _FONTS[key] = ImageFont.truetype(path, size, index=0)
    return _FONTS[key]


def text_w(draw, s, f):
    return draw.textlength(s, font=f)


# ── 语法着色：把一行源码拆成 (text, color) 片段 ────────────────────
KEYWORDS = {"fun", "by", "intro", "exact", "apply", "sorry", "assumption"}


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


def chrome(
    draw: ImageDraw.ImageDraw, filename: str, status: str, status_warn: bool = False
):
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
    draw.text((62, 48), "资源管理器", fill=(200, 200, 200), font=_F("ui"))
    draw.rectangle([58, 84, 224, 108], fill=(58, 58, 58))
    draw.text((70, 88), "PLAYGROUND", fill=(230, 230, 230), font=_F("small"))
    for i, name in enumerate(
        ["playground.sokonanoda", "course/", "examples/", "skills/"]
    ):
        y = 116 + i * 26
        if i == 0:
            draw.rectangle([58, y - 2, 224, y + 22], fill=(47, 61, 76))
        draw.text(
            (78, y),
            name,
            fill=(225, 225, 225) if i == 0 else (170, 170, 170),
            font=_F("small"),
        )
    # 标签栏
    draw.rectangle([EDITOR_X, 34, W, 66], fill=TABBAR_BG)
    draw.rectangle([EDITOR_X, 34, EDITOR_X + 250, 66], fill=EDITOR_BG)
    draw.text((EDITOR_X + 14, 42), filename, fill=(255, 255, 255), font=_F("small"))
    draw.ellipse([EDITOR_X + 232, 46, EDITOR_X + 240, 54], fill=(230, 230, 230))
    # 状态栏
    sb = (170, 110, 20) if status_warn else STATUS_BG
    draw.rectangle([0, H - 26, W, H], fill=sb)
    draw.text((14, H - 21), status, fill=(255, 255, 255), font=_F("small"))
    draw.text(
        (W - 250, H - 21),
        "sokonanoda-lsp  ✓ 内核判卷",
        fill=(255, 255, 255),
        font=_F("small"),
    )


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
        draw.text((16, y + 2), f"{li + 1}", fill=LINE_NO, font=_F("lineno"))
        x = EDITOR_X + PAD_L
        for tok, color in line:
            draw.text((x, y), tok, fill=color, font=_F("code"))
            x += text_w(draw, tok, _F("code"))
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
            x = (
                EDITOR_X
                + PAD_L
                + text_w(draw, "".join(t for t, _ in line), _F("code"))
                + 12
            )
            tw = text_w(draw, label, _F("small"))
            draw.rectangle([x, y - 1, x + tw + 10, y + FS_CODE + 1], fill=(51, 51, 51))
            draw.text(
                (x + 5, y + 1),
                label,
                fill=(150, 170, 200) if ": " in label else OK_GREEN,
                font=_F("small"),
            )
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
    w = (
        max(
            text_w(draw, it["label"], _F("small"))
            + text_w(draw, it.get("detail", ""), _F("small"))
            + 56
            for it in items
        )
        + 20
    )
    h = len(items) * 24 + 8
    draw.rectangle([x, y, x + w, y + h], fill=POPUP_BG, outline=POPUP_BORDER, width=1)
    draw.line([x, y, x, y + h], fill=(0, 122, 204), width=2)
    for i, it in enumerate(items):
        ry = y + 4 + i * 24
        if i == sel:
            draw.rectangle([x + 2, ry - 1, x + w - 2, ry + 21], fill=SELECT_BG)
        # kind 图标
        draw.rectangle(
            [x + 10, ry + 5, x + 22, ry + 17], outline=(200, 160, 220), width=1
        )
        draw.text((x + 28, ry + 1), it["label"], fill=(230, 230, 230), font=_F("small"))
        if it.get("detail"):
            dw = text_w(draw, it["label"], _F("small"))
            draw.text(
                (x + 34 + dw, ry + 1),
                it["detail"],
                fill=(150, 150, 150),
                font=_F("small"),
            )
        if it.get("selected_badge"):
            bw = text_w(draw, it["selected_badge"], _F("small"))
            draw.text(
                (x + w - bw - 12, ry + 1),
                it["selected_badge"],
                fill=(120, 190, 255),
                font=_F("small"),
            )


def _draw_hover(draw, hover, cursor):
    li, col = cursor
    # 显式定位优先（长行上的卡片会溢出画布右缘）。
    x = hover.get("x", EDITOR_X + PAD_L + col + 10)
    y = hover.get("y", 78 + li * LH + LH + 6)
    w = hover["w"]
    h = hover["h"]
    draw.rectangle(
        [x, y, x + w, y + h], fill=(37, 37, 38), outline=(69, 69, 69), width=1
    )
    yy = y + 10
    for line, f, color in hover["lines"]:
        draw.text((x + 12, yy), line, fill=color, font=f)
        yy += f.size + 7
    if hover.get("button"):
        bw = hover["button_w"]
        draw.rectangle([x + 12, yy + 2, x + 12 + bw, yy + 26], fill=(0, 90, 158))
        bt = hover["button"]
        tw = text_w(draw, bt, _F("ui"))
        draw.text(
            (x + 12 + (bw - tw) / 2, yy + 6), bt, fill=(255, 255, 255), font=_F("ui")
        )


# ── 代码行构建 ─────────────────────────────────────────────────────
def L(*parts: tuple[str, tuple] | str) -> list[tuple[str, tuple]]:
    out = []
    for p in parts:
        out.extend((t, c) for t, c in (tokenize(p) if isinstance(p, str) else [p]))
    return out


def line_width(draw, line):
    return text_w(draw, "".join(t for t, _ in line), _F("code"))


# ── 静态图 1：hover `sorry` —— 洞的期望类型 + 剩余目标（0.25.0）──
def demo_goal_png() -> Image.Image:
    """学习者最典型的瞬间：光标落在 `sorry` 上，hover 给出这个洞的精确
    期望类型（经 def `Not` 展开算出）、剩余目标与已引入假设——
    与扩展真实输出逐字一致（`half_expression`/goal walk）。"""
    lines = [
        L("axiom False : Prop"),
        L("axiom And : Prop -> Prop -> Prop"),
        L("axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b"),
        L("def Not : Prop -> Prop := fun (a : Prop) => a -> False"),
        L(""),
        L("theorem and_not_absurd : (a : Prop) -> And a (Not a) -> False :="),
        L(
            "  fun (a : Prop) => fun (x : And a (Not a)) => (And.right a (Not a) x) sorry"
        ),
    ]
    probe = ImageDraw.Draw(Image.new("RGB", (8, 8)))
    prefix = "  fun (a : Prop) => fun (x : And a (Not a)) => (And.right a (Not a) x) "
    sorry_col = int(text_w(probe, prefix, _F("code")))
    img = editor_frame(
        [l[:] for l in lines],
        cursor=(6, sorry_col + int(text_w(probe, "sor", _F("code")))),
        status="练习待填：and_not_absurd",
        status_warn=True,
    )
    draw = ImageDraw.Draw(img)
    hover = {
        # 长行 + 卡片会溢出画布：显式定位在编辑器左上区域。
        "x": EDITOR_X + PAD_L + 20,
        "y": 78 + 0 * LH + LH + 8,
        "w": 470,
        "h": 208,
        "lines": [
            ("此处 `sorry` 的期望类型：", _F("ui"), (230, 230, 230)),
            ("a", _F("code"), OK_GREEN),
            ("", _F("small"), FG),
            ("剩余目标：False", _F("ui"), (230, 230, 230)),
            ("", _F("small"), FG),
            ("已引入假设：", _F("ui"), (200, 200, 200)),
            ("- a : Prop", _F("small"), (170, 200, 230)),
            ("- x : And a (Not a)", _F("small"), (170, 200, 230)),
        ],
    }
    _draw_hover(draw, hover, (6, sorry_col))
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
    x0 = (
        EDITOR_X
        + PAD_L
        + int(text_w(probe, "".join(t for t, _ in wline)[:34], _F("code")))
    )
    img = editor_frame(
        [l[:] for l in lines],
        squiggle=(x0, 0, x0 + 130, 4),
        status="kernel-rejected：内核拒绝",
        status_warn=True,
    )
    draw = ImageDraw.Draw(img)
    # 诊断悬浮框
    hx, hy = EDITOR_X + PAD_L + 60, 78 + 4 * LH + LH + 10
    draw.rectangle(
        [hx, hy, hx + 470, hy + 108], fill=(37, 37, 38), outline=(69, 69, 69), width=1
    )
    draw.line([hx, hy, hx, hy + 108], fill=ERROR, width=2)
    draw.text((hx + 12, hy + 10), "kernel-rejected  ✗", fill=ERROR, font=_F("ui"))
    draw.text(
        (hx + 12, hy + 36),
        "内核拒绝了这个证明：And.intro a b 的结论是",
        font=_F("small"),
        fill=(210, 210, 210),
    ) if False else None
    draw.text(
        (hx + 12, hy + 34),
        "内核拒绝了这个证明。检查提示：",
        fill=(210, 210, 210),
        font=_F("small"),
    )
    draw.text(
        (hx + 12, hy + 58),
        "And.intro b a 的两个前提依次是 b、a——取用的",
        fill=(200, 200, 200),
        font=_F("small"),
    )
    draw.text(
        (hx + 12, hy + 80),
        "顺序写反了。判对判错由内核说了算。",
        fill=(200, 200, 200),
        font=_F("small"),
    )
    return img


# ── 静态图 3：终端 --json 事件流（任意 code agent 的判卷接口）────
STR = (206, 145, 120)  # JSON 字符串（VS Code Dark+ 橙）
KEY = (156, 220, 254)  # JSON 键（浅蓝）
PROMPT = (135, 214, 132)


def demo_agent_png() -> Image.Image:
    """给 code agent 的瞬间：`sokonanoda --json` 每行一个事件——agent 读
    事件判卷，无需读源码。事件形状与 CLI 真实输出逐字一致。"""
    img = Image.new("RGB", (W, H), (18, 18, 18))
    draw = ImageDraw.Draw(img)
    # 终端标题栏
    draw.rectangle([0, 0, W, 34], fill=TITLE_BG)
    for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
        draw.ellipse([14 + i * 22, 12, 26 + i * 22, 24], fill=c)
    title = "agent — sokonanoda --json"
    tw = text_w(draw, title, font(CODE_FONT, 12))
    draw.text(((W - tw) / 2, 10), title, fill=(180, 180, 180), font=font(CODE_FONT, 12))

    f = _F("code")
    fs = _F("small")
    y = 64

    def put(segments, line_h=None):
        nonlocal y
        x = 40
        for text, color, font_ in segments:
            draw.text((x, y), text, fill=color, font=font_)
            x += text_w(draw, text, font_)
        y += line_h or (f.size + 12)

    put(
        [
            ("$ ", PROMPT, f),
            ("sokonanoda --json playground.sokonanoda", (230, 230, 230), f),
        ]
    )
    y += 8
    events = [
        ("type", "decl.checked", "checked declaration true_is_true", "true_is_true"),
        (
            "type",
            "decl.checked",
            "checked declaration and_intro_rule",
            "and_intro_rule",
        ),
        ("type", "decl.checked", "checked declaration and_swap", "and_swap"),
        ("type", "exercise.open", "exercise open (fill the sorry)", "and_not_absurd"),
    ]
    for typ, t, human, name in events:
        sep = '","'
        put(
            [
                ('{"', PUNCT, fs),
                ("human", KEY, fs),
                ('":"', PUNCT, fs),
                (human + sep, STR, fs),
                ("name", KEY, fs),
                ('":"', PUNCT, fs),
                (name + sep, STR, fs),
                ("type", KEY, fs),
                ('":"', PUNCT, fs),
                (t + '"}', STR, fs),
            ]
        )
    y += 14
    put(
        [
            ("# agent 读事件，不做文本比对：decl.checked = 解出，", COMMENT, fs),
        ],
        fs.size + 10,
    )
    put(
        [
            ("# exercise.open = 进行中（指向 soko:hint 阶梯）。", COMMENT, fs),
        ],
        fs.size + 10,
    )
    put([("$ ", PROMPT, f), ("█", CURSOR, f)])
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
        append_images=[
            im.convert("P", palette=Image.ADAPTIVE, colors=64) for im in ims[1:]
        ],
        duration=durs,
        loop=0,
        optimize=True,
    )


def render_all() -> dict[str, list[tuple[Image.Image, int]]]:
    """全部演示的帧序列（单一事实源：页面与 --check 都从这里出发）。"""
    return {
        "demo-goal.png": [(demo_goal_png(), 0)],
        "demo-kernel.png": [(demo_kernel_png(), 0)],
        "demo-agent.png": [(demo_agent_png(), 0)],
    }


def _quantized_rgb(img: Image.Image) -> Image.Image:
    # 与 save_gif 的调色板量化同口径：GIF 解码出来的像素 = 量化后的像素。
    return img.convert("P", palette=Image.ADAPTIVE, colors=64).convert("RGB")


def frame_fingerprint(frames: list[tuple[Image.Image, int]]) -> list[str]:
    return [hashlib.sha256(_quantized_rgb(f).tobytes()).hexdigest() for f, _ in frames]


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
