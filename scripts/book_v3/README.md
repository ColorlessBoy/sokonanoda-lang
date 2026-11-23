# Book v3 — 多页书页（builder + 内容源，生成页不进 git）

把 `courses/analysis/` 的 171 个单元渲染成一本书：封面、卷/章目录、每单元一页、
搜索、sitemap、打印页。**生成页是构建产物**（`dist/book/`，已 gitignore），
仓库里只有 builder 与内容源。

## 两层

| 层 | 脚本 | 产出 |
|---|---|---|
| v2 单页 | `scripts/build_book_proto.py` | 一门课一页自包含 HTML（单元正文/练习/hint/解答 + 解答附录） |
| v3 多页 | `scripts/book_v3/build_site.py` | 从 v2 页切出片段，重组为每单元一页 + 章页 + 卷封面 + 搜索/sitemap/打印 |

`scripts/build_book_v3.py` 是入口：跑 v2、对每个卷跑 v3、再放库首页与卷 0 导论。

## 构建 / 校验（仓库根，只要 Python 3）

```bash
python3 scripts/build_book_v3.py            # 构建 → dist/book/book-v3/
python3 scripts/build_book_v3.py --check    # 校验：每个相对链接可解析 + 单元/章页数 ⇄ course.json
python3 scripts/build_book_v3.py --volume volume-ii   # 只构建一卷
```

单独跑 v2（调试用）：

```bash
python3 scripts/build_book_proto.py --out dist/book/book-v2
python3 scripts/build_book_proto.py --selftest   # 需要 scripts/content_style/（未随本目录提供）
```

## 范围

- **卷 II 数系与实数**：`courses/analysis`，171 单元 / 16 章（II.1–III.9），已可出版。
- **卷 I 集合论**：`courses/set-theory` 的单元文件目前过不了 v2 渲染契约（4 处未解析行、
  2 处 Markdown 标记、I.10–I.13 单元号不连续），需要一轮内容修订后再加回
  `scripts/build_book_v3.py` 的 `VOLUMES`。
- 生成页与截图都不入库；CI（`.github/workflows/book.yml`）构建并上传 `book-v3-html` 产物。

## 目录

```
scripts/book_v3/
  build_site.py         v3 构建器（片段重组、模板渲染、链接改写、--check）
  templates/            cover / chapter / unit / print / search / sitemap / 404
  assets/               v3.css · v3.js
  static/               库首页（两卷落地页）与卷 0「形式逻辑」导论（手写页）
```
