// 记法输入法表：`\and` → `∧`（设计 `docs/design/notation-input.md` §2/§5，NI-2）。
//
// **这是 `crates/front/src/notation_input.rs` 的 `TABLE` 的镜像，不是第二份真相源。**
// LSP 的 hover 文案（「输入：`\and`（别名 `\wedge`）」）从 Rust 表渲染；这份 JS 表
// 只服务 VS Code 的缩写改写器（`abbreviation-rewriter.js`）。两边的
// symbol / abbreviation / aliases / supported / **顺序** 由
// `crates/cli/tests/extension.rs::abbreviation_table_mirrors_the_single_source`
// 用 serde_json **真解析**后逐条比对（与
// `tm_grammar_keywords_follow_the_single_source` 同一形制）——改一边不改另一边，
// `cargo test -p sokonanoda-cli --test extension` 直接红。
//
// 为什么表是「一条一行、键带引号」的**纯 JSON 数组字面量**：Rust 侧要能真解析它，
// 不做 grep 掩膜（硬规则 4：判定不许靠文本比对）。**格式即契约，别重排。**
//
// `''`（像）**没有条目**——与 Lean 4 一致（直接打两个单引号），设计 §2。
"use strict";

const TABLE = [
  { "symbol": "∧", "abbreviation": "and", "aliases": ["wedge"], "supported": true },
  { "symbol": "∨", "abbreviation": "or", "aliases": ["vee"], "supported": true },
  { "symbol": "↔", "abbreviation": "iff", "aliases": ["leftrightarrow"], "supported": true },
  { "symbol": "¬", "abbreviation": "not", "aliases": ["neg"], "supported": true },
  { "symbol": "→", "abbreviation": "to", "aliases": ["imp"], "supported": true },
  { "symbol": "∀", "abbreviation": "forall", "aliases": [], "supported": true },
  { "symbol": "∃", "abbreviation": "exists", "aliases": [], "supported": true },
  { "symbol": "≠", "abbreviation": "ne", "aliases": ["neq"], "supported": true },
  { "symbol": "∈", "abbreviation": "in", "aliases": ["mem"], "supported": true },
  { "symbol": "⊆", "abbreviation": "sub", "aliases": ["subseteq"], "supported": true },
  { "symbol": "∪", "abbreviation": "cup", "aliases": ["union"], "supported": true },
  { "symbol": "∩", "abbreviation": "cap", "aliases": ["inter"], "supported": true },
  { "symbol": "\\", "abbreviation": "setminus", "aliases": [], "supported": true },
  { "symbol": "∅", "abbreviation": "empty", "aliases": ["emptyset"], "supported": true },
  { "symbol": "𝒫", "abbreviation": "powerset", "aliases": [], "supported": true },
  { "symbol": "ᶜ", "abbreviation": "compl", "aliases": ["complement"], "supported": true },
  { "symbol": "⁻¹'", "abbreviation": "preim", "aliases": ["preimage"], "supported": true },
  { "symbol": "×ˢ", "abbreviation": "xs", "aliases": [], "supported": true },
];

// 缩写（含别名）→ 符号。`supported: false` 的条目不收：语言今天没有这个符号，
// 改写器不许把它敲出来（与 Rust 侧 `symbol_for_abbreviation` 同一口径）。
// **大小写敏感**（与 Lean 一致：`\And` 不转换）。
const BY_ABBREVIATION = new Map();
for (const entry of TABLE) {
  if (!entry.supported) continue;
  for (const abbreviation of [entry.abbreviation, ...entry.aliases]) {
    BY_ABBREVIATION.set(abbreviation, entry.symbol);
  }
}

// 表中全部缩写（含别名、含 `supported: false` 的）——只用于前缀判断。
const ALL_ABBREVIATIONS = [];
for (const entry of TABLE) {
  ALL_ABBREVIATIONS.push(entry.abbreviation, ...entry.aliases);
}

function symbolForAbbreviation(abbreviation) {
  return BY_ABBREVIATION.get(abbreviation);
}

// 这个缩写是不是**更长缩写的真前缀**（`in` ⊂ `inter`）？
// 即时替换必须等它不再增长才落定（设计 §3 方案 A 的前缀陷阱：`\in` 是 `\inter`
// 的前缀，不等就会把 `\inter` 打成 `∩ter`）。Tab 是显式命令，不受这条限制。
function isIncomplete(abbreviation) {
  return ALL_ABBREVIATIONS.some(
    (other) => other !== abbreviation && other.startsWith(abbreviation),
  );
}

module.exports = { TABLE, ALL_ABBREVIATIONS, symbolForAbbreviation, isIncomplete };
