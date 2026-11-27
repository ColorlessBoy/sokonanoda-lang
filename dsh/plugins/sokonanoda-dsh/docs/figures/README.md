# 课堂笔记图解

这里只放**知识卡到图解**的白名单资产：[`lib/class-notes-figures.mjs`](../../lib/class-notes-figures.mjs)
点名哪些卡片用哪张 PNG；`GET /sokonanoda-notes-asset?path=<name>.png` 只服务这些文件名
（其它路径、其它后缀、`..` 一律拒绝）。

重新生成（TikZ → PDF → PNG，220 dpi）：

```bash
cd dsh/plugins/sokonanoda-dsh/docs/figures
tectonic -X compile injective.tex
pdftoppm -png -r 220 -singlefile injective.pdf injective
tectonic -X compile set-ext.tex
pdftoppm -png -r 220 -singlefile set-ext.pdf set-ext
```

| 文件 | 卡片 | 内容 |
|---|---|---|
| `injective.png` | `injective` | 单射：不同输入到达不同输出 |
| `set-ext.png` | `set-ext` | 外延性：互相包含 ⇒ 相等 |

图是数学示意，不替代内核判定；文字用英文数学标签，中文说明在笔记正文与 `figcaption`。
