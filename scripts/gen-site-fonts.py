#!/usr/bin/env python3
"""生成官网自托管的 webfont 子集（`site/assets/fonts/*.woff2`）与 `site/assets/fonts.css`。

设计与全部实测数据见 `docs/design/site-rebuild/research/R4-fonts.md`。三条硬约束：

1. **字形集是实测出来的，不是猜的**：`REQUIRED` 是显式清单（可复现），
   `--audit` 会拿它去比对仓库真实内容，内容漂移就报错。
2. **覆盖率靠解析 cmap 判定，不靠希望**：每个产物写完都重新用 fontTools
   打开、逐码位断言，缺一个就是 FAIL（不是 warning）。
3. **可复现**：源字体按 commit/tag + sha256 双重钉死，缓存进 `.cache/fonts/src/`；
   重跑字节一致。

依赖 fontTools + brotli（**仅维护者**，学习者路径零工具链不受影响）：

    pip3 install --target .cache/pylibs fonttools brotli

用法：

    python3 scripts/gen-site-fonts.py              # 生成（幂等）
    python3 scripts/gen-site-fonts.py --check      # 只校验，不写盘（CI 用）
    python3 scripts/gen-site-fonts.py --audit      # 额外做仓库内容漂移审计
    python3 scripts/gen-site-fonts.py --update-sources   # 重新下载并打印新的 sha256
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC_CACHE = ROOT / ".cache" / "fonts" / "src"
OUT_DIR = ROOT / "site" / "assets" / "fonts"
CSS_PATH = ROOT / "site" / "assets" / "fonts.css"

# ── 依赖装载 ────────────────────────────────────────────────────────────────


def load_fonttools():
    """把 `.cache/pylibs` 挂到 sys.path 上再 import；缺依赖时给人话。"""
    override = os.environ.get("SOKO_PYLIBS")
    lib = Path(override) if override else ROOT / ".cache" / "pylibs"
    if lib.is_dir() and str(lib) not in sys.path:
        sys.path.insert(0, str(lib))
    try:
        import brotli  # noqa: F401  (woff2 压缩后端，fontTools 需要)
        from fontTools import subset
        from fontTools.ttLib import TTFont
        from fontTools.varLib import instancer
    except ImportError as exc:  # pragma: no cover - 维护者环境问题
        sys.stderr.write(
            "gen-site-fonts: 缺少依赖 {name}（{exc}）。\n"
            "\n"
            "这是维护者工具，需要 fontTools + brotli：\n"
            "\n"
            "    pip3 install --target .cache/pylibs fonttools brotli\n"
            "\n"
            "产物（site/assets/fonts/*.woff2、site/assets/fonts.css）已提交入库，\n"
            "普通使用/学习路径不需要跑本脚本。\n".format(
                name=getattr(exc, "name", "fontTools/brotli"), exc=exc
            )
        )
        raise SystemExit(2)
    return TTFont, subset, instancer


# ── 字形集（Step 1 实测，见 R4-fonts.md §2）─────────────────────────────────

ASCII = tuple(range(0x20, 0x7F))  # 可打印 ASCII

# 扫描仓库真实内容得到的非 ASCII、非 CJK、非全角码位（68 个）。
# 复现命令见 R4-fonts.md §2；--audit 会用同一组 glob 重新扫。
MEASURED = (
    0x00A7, 0x00AC, 0x00B7, 0x00B9, 0x00D7, 0x00F6, 0x02E2, 0x03A0, 0x03A9,
    0x03B1, 0x03B2, 0x03B3, 0x03B4, 0x03BB, 0x03C6, 0x1D9C, 0x2013, 0x2014,
    0x201C, 0x201D, 0x2022, 0x2026, 0x207B, 0x2115, 0x211D, 0x2160, 0x2161,
    0x2190, 0x2192, 0x2193, 0x2194, 0x21AA, 0x21CF, 0x21D2, 0x21D4, 0x2200,
    0x2203, 0x2205, 0x2208, 0x2209, 0x220E, 0x2212, 0x2218, 0x2227, 0x2228,
    0x2229, 0x222A, 0x2241, 0x2248, 0x2260, 0x2264, 0x2265, 0x2286, 0x2287,
    0x22A2, 0x2460, 0x2461, 0x2462, 0x2463, 0x2464, 0x2465, 0x2466, 0x2467,
    0x2468, 0x2469, 0x246A, 0x246B, 0x2500, 0x2502, 0x2514, 0x251C, 0x2550,
    0x25BC, 0x2605, 0x26A0, 0x2713, 0x27E8, 0x27E9, 0x27F9, 0x27FA, 0x2983,
    0x2984, 0x1D4AB,
)

# 任务书点名、且排版上确实要用的补充符号。
# U+2018/U+2019：英文撇号/单引号。仓库当前内容里实测 0 次（内容用的是 ASCII '），
# 但中英混排正文一旦出现 “kernel’s” 这类写法，缺字会掉进 CJK 字体、骨架不齐，
# 两个码位约 600 B，作为预防性覆盖纳入。
TYPO_EXTRA = (0x2018, 0x2019)

REQUIRED = tuple(sorted(set(ASCII) | set(MEASURED) | set(TYPO_EXTRA)))

# 等宽面必须独立覆盖全集：目标态面板是等宽的，缺一个字形就逐字回退、对齐崩掉。
MONO_REQUIRED = REQUIRED

# ── 源字体（URL 与 sha256 双重钉死）────────────────────────────────────────

GF = "https://cdn.jsdelivr.net/gh/google/fonts@f2bd09badbc763d8757951d52deec29da27e85fb"
JM = "https://cdn.jsdelivr.net/gh/cormullion/juliamono@v0.059"
SX = "https://cdn.jsdelivr.net/gh/stipub/stixfonts@v2.13/fonts/static_otf"

SOURCES = {
    # 正文/标题拉丁面：Source Serif 4（SIL OFL 1.1），可变字体
    "SourceSerif4.ttf": (
        GF + "/ofl/sourceserif4/SourceSerif4%5Bopsz,wght%5D.ttf",
        "97b2d4da6e3cb494b5a1e66ae176914d852ccabef49e0c02c0df25f3e39aca0b",
    ),
    "SourceSerif4-Italic.ttf": (
        GF + "/ofl/sourceserif4/SourceSerif4-Italic%5Bopsz,wght%5D.ttf",
        "15fbc7e4679489a501998c3669272637a6646388ef7e4bd77eebb5bf967a1f42",
    ),
    # 等宽面：JuliaMono（SIL OFL 1.1）——唯一 100% 覆盖本项目符号集的候选
    "JuliaMono-Regular.ttf": (
        JM + "/JuliaMono-Regular.ttf",
        "3e521304357b22b90c02d003e8fa4fb7b49c1267e6459240fd06b2d1900e36c1",
    ),
    "JuliaMono-Bold.ttf": (
        JM + "/JuliaMono-Bold.ttf",
        "f4605d773f435cfcba9220cb81a4e3b8fa6a2b4002cb62248d020500587ffdaf",
    ),
    # 符号兜底面：STIX Two Math（SIL OFL 1.1）
    "STIXTwoMath-Regular.otf": (
        SX + "/STIXTwoMath-Regular.otf",
        "f2076b9f1676438439dd41e23676f5ab99056e83d6b8f8c27841591ef2ccfa72",
    ),
}

# 衬线面保留的 OpenType 特性：kern/liga 是排版质量，tnum/zero/lnum 是 R1 §3.6
# 要求的等宽数字与带斜杠零；**不**保留 ss01-ss20 / cv01-* 等风格集（实测占 ~27 KB）。
SERIF_FEATURES = ["kern", "liga", "ccmp", "locl", "mark", "mkmk", "calt", "tnum", "zero", "lnum"]

# 输出面定义。`symbols` 面的码位集在运行时由衬线面 cmap 反推（不硬编码）。
FACES = (
    {
        "file": "soko-serif-roman.woff2",
        "src": "SourceSerif4.ttf",
        "instance": {"opsz": 20, "wght": (400, 700)},
        "layout": SERIF_FEATURES,
        "glyphs": "required",
        "role": "正文/标题（拉丁），可变字重 400–700",
    },
    {
        "file": "soko-serif-italic.woff2",
        "src": "SourceSerif4-Italic.ttf",
        "instance": {"opsz": 20, "wght": (400, 700)},
        "layout": SERIF_FEATURES,
        "glyphs": "required",
        "order": "subset-first",
        "role": "强调与中文正文里的拉丁术语（真斜体）",
    },
    {
        "file": "soko-mono-400.woff2",
        "src": "JuliaMono-Regular.ttf",
        "layout": [],
        "glyphs": "required",
        "role": "代码块/终端/目标态面板",
    },
    {
        "file": "soko-mono-700.woff2",
        "src": "JuliaMono-Bold.ttf",
        "layout": [],
        "glyphs": "required",
        "role": "等宽粗体（面板标题、强调）",
    },
    {
        "file": "soko-symbols.woff2",
        "src": "STIXTwoMath-Regular.otf",
        "layout": [],
        "glyphs": "serif-missing",
        "role": "衬线面缺的数学/圈码符号兜底（unicode-range 限定，按需下载）",
    },
)

CSS_FAMILY = {
    "soko-serif-roman.woff2": ("Soko Serif", "normal", "400 700"),
    "soko-serif-italic.woff2": ("Soko Serif", "italic", "400 700"),
    "soko-mono-400.woff2": ("Soko Mono", "normal", "400"),
    "soko-mono-700.woff2": ("Soko Mono", "normal", "700"),
    "soko-symbols.woff2": ("Soko Symbols", "normal", "400"),
}

# ── 仓库内容审计（--audit）────────────────────────────────────────────────

# 实测扫到但**故意不子集**的码位：变体选择符是零宽格式字符，本身不需要字形
# （U+FE0F 出现在 `⚠️` 里，向渲染器请求 emoji 呈现；由系统 emoji/符号回退链处理，
# 放进 Latin 子集只会白白多一个空字形）。审计会把这些单独列出来，不静默吞掉。
IGNORED_CODEPOINTS = frozenset(range(0xFE00, 0xFE10)) | {0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF}

AUDIT_GLOBS = (
    "site/**/*.html",
    "site/**/*.js",
    "site/**/*.css",
    "site/**/*.json",
    "playground.sokonanoda",
    "examples/*.sokonanoda",
    "courses/set-theory/lib/*.sokonanoda",
    "courses/set-theory/units/*.sokonanoda",
    "course/*.sokonanoda",
    "README.md",
    "docs/protocol.md",
    "docs/architecture.md",
)


def strip_non_rendered(text: str, suffix: str) -> str:
    """去掉不可能上屏的内容：CSS 的 /* */ 与 HTML 的 <!-- -->。

    审计只关心「能到达屏幕的字符」。实测例子：`site/index.html` 里 206 个 `═`
    全在 HTML 注释里；`site/assets/tokens.css` 的注释里用 ✅ 标注对比度结论。
    把它们算进字形集只会让清单虚胖（还得为 emoji 兜底，而 emoji 一律走平台
    字体）。真正的 `content: "✅"` 不在注释里，仍然会被抓到。
    """
    import re

    if suffix == ".css":
        return re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    if suffix in (".html", ".htm"):
        return re.sub(r"<!--.*?-->", " ", text, flags=re.S)
    return text


def scan_repo_codepoints() -> dict[int, str]:
    """按 R4-fonts.md §2 的口径重扫仓库，返回 {码位: 首个出现文件}。

    跳过本脚本自己的产物：`site/assets/fonts.css` 的注释里有字体作者名
    （Grießhammer 的 ß），`site/assets/fonts/*.woff2` 是二进制——它们不是
    「站点内容」，不该反过来定义字形集。
    """
    import glob

    skip = {CSS_PATH.resolve(), *(p.resolve() for p in OUT_DIR.glob("*"))}
    found: dict[int, str] = {}
    for pattern in AUDIT_GLOBS:
        for name in sorted(glob.glob(str(ROOT / pattern), recursive=True)):
            path = Path(name)
            if not path.is_file() or path.resolve() in skip:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            text = strip_non_rendered(text, path.suffix)
            for ch in text:
                cp = ord(ch)
                if cp < 0x00A0:
                    continue
                if 0x3000 <= cp <= 0x9FFF or 0xFF00 <= cp <= 0xFFEF:  # CJK / 全角
                    continue
                found.setdefault(cp, str(path.relative_to(ROOT)))
    return found


# ── 工具 ───────────────────────────────────────────────────────────────────


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def fetch(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "sokonanoda-gen-site-fonts/1"})
    with urllib.request.urlopen(req, timeout=180) as resp:  # noqa: S310 (固定 https 白名单)
        return resp.read()


def ensure_source(name: str, update: bool) -> Path:
    """取回并校验源字体；命中缓存且 sha256 一致就不重复下载。"""
    url, want = SOURCES[name]
    SRC_CACHE.mkdir(parents=True, exist_ok=True)
    dest = SRC_CACHE / name
    if dest.is_file() and not update and sha256(dest) == want:
        return dest
    sys.stdout.write(f"  download {name} <- {url}\n")
    data = fetch(url)
    got = hashlib.sha256(data).hexdigest()
    if got != want:
        if update:
            sys.stdout.write(f"  UPDATE-PIN {name} sha256={got}\n")
        else:
            sys.stderr.write(
                f"gen-site-fonts: {name} 的 sha256 与钉住的值不一致——上游动了。\n"
                f"  url       {url}\n  期望      {want}\n  实际      {got}\n"
                "产物不会静默改变。确认上游改动无误后跑 "
                "`python3 scripts/gen-site-fonts.py --update-sources` 取新值。\n"
            )
            raise SystemExit(1)
    dest.write_bytes(data)
    return dest


def unicode_range_css(codepoints) -> str:
    return ", ".join(f"U+{cp:04X}" for cp in sorted(codepoints))


def build_face(TTFont, subset_mod, instancer, spec, src: Path, unicodes, out: Path) -> int:
    font = TTFont(str(src), fontNumber=0)
    opts = subset_mod.Options()
    opts.layout_features = list(spec.get("layout", []))
    # 只留身份/许可相关的 name 记录（OFL 要求保留版权与许可声明）
    opts.name_IDs = [0, 1, 2, 3, 4, 5, 6, 7, 13, 14, 16, 17]
    opts.name_legacy = True
    opts.name_languages = ["*"]
    opts.notdef_outline = True
    opts.hinting = False
    opts.glyph_names = False
    opts.recalc_timestamp = False  # 字节级幂等
    opts.canonical_order = True
    opts.drop_tables = sorted(set(opts.drop_tables) | {"DSIG", "MATH"})
    opts.flavor = "woff2"

    instance = spec.get("instance")
    if instance and spec.get("order") != "subset-first":
        instancer.instantiateVariableFont(font, instance, inplace=True, updateFontNames=False)

    sub = subset_mod.Subsetter(options=opts)
    sub.populate(unicodes=list(unicodes))
    sub.subset(font)

    if instance and spec.get("order") == "subset-first":
        # Source Serif 4 Italic 先实例化会触发 gvar 引用了被裁掉字形的 KeyError，
        # 反过来（先子集再实例化）等价且稳定。
        instancer.instantiateVariableFont(font, instance, inplace=True, updateFontNames=False)

    font.flavor = "woff2"
    out.parent.mkdir(parents=True, exist_ok=True)
    # TTFont 默认 recalcTimestamp=True，_save() 会把 head.modified 写成「现在」——
    # 这是唯一的非确定性来源（实测两次运行只差 head 里两个字节）。关掉它产物才
    # 字节级幂等，head.modified 保留上游字体自己的时间戳。
    # 注意 fontTools 4.65 的 TTFont.save() 已经没有 recalcTimestamp 参数了，
    # 必须走实例属性。
    font.recalcTimestamp = False
    font.save(str(out))
    font.close()
    return out.stat().st_size


def read_cmap(TTFont, path: Path) -> set[int]:
    font = TTFont(str(path))
    try:
        return set(font.getBestCmap())
    finally:
        font.close()


def verify_face(TTFont, path: Path, expected) -> list[int]:
    """重新打开产物，逐码位断言。返回缺失码位。

    这里的 `expected` 是 **源字体本来就有、因此必须出现在产物里** 的码位——
    子集器悄悄丢字形会被这一条抓住（是 FAIL，不是 warning）。
    """
    cmap = read_cmap(TTFont, path)
    return [cp for cp in expected if cp not in cmap]


def check_css_links(css_text: str) -> list[str]:
    """fonts.css 与 fonts/ 目录必须互相自洽：

    * 每条 `url(...)` 相对 CSS 所在目录解析后必须真实存在；
    * `site/assets/fonts/` 里每个文件都必须至少被一条 `url(...)` 引用。
    """
    import re

    problems: list[str] = []
    referenced: set[str] = set()
    for raw in re.findall(r'url\(\s*["\']?([^"\')]+)["\']?\s*\)', css_text):
        if raw.startswith(("data:", "http:", "https:", "//")):
            continue
        target = (CSS_PATH.parent / raw).resolve()
        referenced.add(target.name)
        if not target.is_file():
            problems.append(f"url({raw!r}) 解析到 {target}，文件不存在")
    on_disk = {p.name for p in OUT_DIR.glob("*") if p.is_file()}
    for name in sorted(on_disk - referenced):
        problems.append(f"site/assets/fonts/{name} 没有被 fonts.css 里任何 url() 引用")
    for name in sorted(referenced - on_disk):
        problems.append(f"fonts.css 引用了 {name}，但 site/assets/fonts/ 里没有")
    return problems


def render_css(rows) -> str:
    lines = [
        "/* site/assets/fonts.css — 由 scripts/gen-site-fonts.py 生成，请勿手改。",
        " *",
        " * 自托管 Latin 子集（woff2 only）。CJK **不**自托管，走系统栈（见下）。",
        " * 依据与全部实测数据：docs/design/site-rebuild/research/R4-fonts.md",
        " *",
        " * 字体与许可证（均为 SIL OFL 1.1，允许子集化与再分发）：",
        ' *   "Soko Serif"        ← Source Serif 4      (Frank Grießhammer / Adobe)',
        ' *   "Soko Mono"         ← JuliaMono           (Cormullion)',
        ' *   "Soko Symbols"      ← STIX Two Math       (STI Pub Companies / Tiro Typeworks)',
        " *",
        " * 字形集：可打印 ASCII + 仓库实测的 85 个符号（含 ⊢ U+22A2、𝒫 U+1D4AB）。",
        " * 每个面的码位都是显式清单，不是 unicode-range 块；只有 Soko Symbols 面",
        " * 带 unicode-range（它就是按「衬线面缺哪些」切的），浏览器因此可以按需下载。",
        " *",
        " * 覆盖：Soko Serif 自带 129/180；缺的 51 个由 Soko Symbols（STIX Two Math，",
        " * 49 个）与 Soko Mono（JuliaMono，51/51）兜底。所以正文栈必须按",
        " * Serif → Symbols → Mono 的顺序写，缺字永远不会掉到平台字体上。",
        " */",
        "",
    ]
    for row in rows:
        family, style, weight = CSS_FAMILY[row["file"]]
        lines.append("@font-face {")
        lines.append(f'  font-family: "{family}";')
        lines.append(f"  font-style: {style};")
        lines.append(f"  font-weight: {weight};")
        lines.append("  font-display: swap;")
        if row.get("range"):
            lines.append(f"  unicode-range: {unicode_range_css(row['range'])};")
        lines.append(f'  src: url("fonts/{row["file"]}") format("woff2");')
        lines.append("}")
        lines.append("")
    lines += [
        "/* ── 字体栈（本文件自带一份参考实现，用 --soko-font-* 命名空间，",
        " *   不与 style.css / tokens.css 的 --font-* 抢名字；站点实际令牌以",
        " *   site/assets/tokens.css 为准，这里只保证单独链 fonts.css 也能用）──",
        " *",
        " * CJK 走系统栈，逐平台实际命中：",
        " *   macOS   PingFang SC（10.11+；`Hiragino Sans GB` 是 10.6–10.10 的旧名，留作回退）",
        " *   Windows Microsoft YaHei UI → Microsoft YaHei（Win7+；中文正文标准黑体）",
        " *   Linux   Noto Sans CJK SC（发行版常装），否则 Source Han Sans SC，最后 sans-serif",
        " * `system-ui` 排最前，让非 Apple 平台也先吃系统 UI 字体。",
        " *",
        " * Serif → Symbols → Mono 的顺序是有意的：Mono 100% 覆盖符号集，是最后一道",
        " * 自托管保险；它本来就要为代码块下载，所以兜底不产生额外请求。",
        " */",
        ":root {",
        "  --soko-font-prose:",
        '    "Soko Serif", "Soko Symbols", "Soko Mono", system-ui, -apple-system,',
        '    "Segoe UI", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI",',
        '    "Microsoft YaHei", "Noto Sans CJK SC", "Source Han Sans SC", sans-serif;',
        "  --soko-font-mono:",
        '    "Soko Mono", ui-monospace, SFMono-Regular, Menlo, Consolas,',
        '    "Liberation Mono", "DejaVu Sans Mono", monospace;',
        "  --soko-font-cjk:",
        '    "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI", "Microsoft YaHei",',
        '    "Noto Sans CJK SC", "Source Han Sans SC", "Noto Sans SC", sans-serif;',
        "}",
        "",
    ]
    return "\n".join(lines)


# ── 主流程 ─────────────────────────────────────────────────────────────────


def main() -> int:
    ap = argparse.ArgumentParser(description="生成官网自托管 webfont 子集与 @font-face")
    ap.add_argument("--check", action="store_true", help="只校验，不写盘")
    ap.add_argument("--audit", action="store_true", help="额外审计仓库内容漂移")
    ap.add_argument("--update-sources", action="store_true", help="重新下载源字体并打印新 sha256")
    args = ap.parse_args()

    TTFont, subset_mod, instancer = load_fonttools()

    if args.audit:
        found = scan_repo_codepoints()
        known = set(REQUIRED) | IGNORED_CODEPOINTS
        drift = {cp: f for cp, f in found.items() if cp not in known}
        ignored = sorted(cp for cp in found if cp in IGNORED_CODEPOINTS)
        print(f"audit: 仓库扫描到 {len(found)} 个码位，显式清单 {len(REQUIRED)} 个")
        if ignored:
            print(
                "audit: 故意忽略 "
                + ", ".join(f"U+{cp:04X}" for cp in ignored)
                + "（零宽格式字符，不需要字形）"
            )
        if drift:
            print("audit: FAIL —— 下列码位在仓库里出现但不在 REQUIRED 里：")
            for cp, f in sorted(drift.items()):
                print(f"  U+{cp:04X} {chr(cp)!r}  首次出现 {f}")
            return 1
        print("audit: PASS —— 仓库内容没有超出显式清单")

    print("source fonts (.cache/fonts/src/):")
    paths = {name: ensure_source(name, args.update_sources) for name in SOURCES}

    # 衬线面缺哪些 → 符号兜底面的候选码位（运行时反推，不硬编码）
    serif_cmap = read_cmap(TTFont, paths["SourceSerif4.ttf"])
    serif_missing = tuple(cp for cp in REQUIRED if cp not in serif_cmap)
    print(
        f"serif coverage: Source Serif 4 覆盖 {len(REQUIRED) - len(serif_missing)}/{len(REQUIRED)}，"
        f"缺 {len(serif_missing)} 个"
    )

    glyph_sets = {"required": REQUIRED, "serif-missing": serif_missing}
    rows = []
    failures = []

    print()
    print(f"{'file':26s} {'bytes':>8s} {'asked':>6s} {'carried':>8s} {'dropped':>8s}  result")
    for spec in FACES:
        out = OUT_DIR / spec["file"]
        requested = glyph_sets[spec["glyphs"]]
        src_cmap = read_cmap(TTFont, paths[spec["src"]])
        # 源字体本来就有的才可能进产物；这是「不许静默丢字形」的判据。
        expected = tuple(cp for cp in requested if cp in src_cmap)
        size = build_face(TTFont, subset_mod, instancer, spec, paths[spec["src"]], requested, out)
        cmap = read_cmap(TTFont, out)
        dropped = verify_face(TTFont, out, expected)
        carried = tuple(cp for cp in requested if cp in cmap)
        ok = not dropped
        print(
            f"{spec['file']:26s} {size:8d} {len(requested):6d} {len(carried):8d} "
            f"{len(requested) - len(carried):8d}  {'PASS' if ok else 'FAIL'}"
        )
        if not ok:
            failures.append((spec["file"], dropped))
            print("      SILENTLY DROPPED: " + " ".join(f"U+{cp:04X}" for cp in dropped))
        rows.append(
            {
                "file": spec["file"],
                "bytes": size,
                "cmap": cmap,
                "range": tuple(sorted(carried)) if spec["glyphs"] != "required" else None,
            }
        )

    total = sum(r["bytes"] for r in rows)
    print(f"\ntotal payload: {total} B = {total / 1024:.1f} KB over {len(rows)} files")

    # 覆盖判据：自托管面的并集必须覆盖 REQUIRED，且等宽面必须**单独**覆盖全集。
    by_file = {r["file"]: r["cmap"] for r in rows}
    union = set().union(*by_file.values())
    gap = [cp for cp in REQUIRED if cp not in union]
    mono_gap = [cp for cp in MONO_REQUIRED if cp not in by_file["soko-mono-400.woff2"]]
    serif_only = [cp for cp in REQUIRED if cp in by_file["soko-serif-roman.woff2"]]
    print(
        f"union  自托管面并集覆盖 REQUIRED:  {'PASS' if not gap else 'FAIL ' + str(gap)}"
        f"  ({len(union & set(REQUIRED))}/{len(REQUIRED)})"
    )
    print(
        f"mono   Soko Mono 单独覆盖 REQUIRED: {'PASS' if not mono_gap else 'FAIL ' + str(mono_gap)}"
        f"  ({len(MONO_REQUIRED) - len(mono_gap)}/{len(MONO_REQUIRED)})"
    )
    print(
        f"split  衬线面自带 {len(serif_only)} 个；其余 {len(REQUIRED) - len(serif_only)} 个由 "
        "Soko Symbols → Soko Mono 兜底（CSS 栈里顺序固定，不依赖平台字体）"
    )
    union_ok = not gap and not mono_gap

    css_text = render_css(rows)
    links = check_css_links(css_text)
    if links:
        for p in links:
            print(f"css    FAIL {p}")
    else:
        print(
            f"css    url() 与 site/assets/fonts/ 自洽: PASS "
            f"（{len(rows)} 条 url()，{len(rows)} 个文件）"
        )

    if args.check:
        current = CSS_PATH.read_text(encoding="utf-8") if CSS_PATH.is_file() else ""
        css_ok = current == css_text
        print(f"css    site/assets/fonts.css up-to-date: {'PASS' if css_ok else 'FAIL (run without --check)'}")
        return 0 if (union_ok and not failures and css_ok and not links) else 1

    if CSS_PATH.is_file() and CSS_PATH.read_text(encoding="utf-8") == css_text:
        print("css    site/assets/fonts.css up-to-date")
    else:
        CSS_PATH.parent.mkdir(parents=True, exist_ok=True)
        CSS_PATH.write_text(css_text, encoding="utf-8")
        print(f"css    wrote site/assets/fonts.css ({len(css_text.encode('utf-8'))} B)")

    if failures or links:
        sys.stderr.write("gen-site-fonts: 有面缺字形或 CSS 链接不自洽，产物不可用\n")
        return 1
    return 0 if union_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
