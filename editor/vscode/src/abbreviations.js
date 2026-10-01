// 记法输入法表：`\and` → `∧`、`\alpha` → `α`（设计 `docs/design/notation-input.md`
// §3 的 D1–D4）。
//
// **这是 `crates/front/src/notation_input.rs` 的 `TABLE` 的镜像，不是第二份真相源。**
// LSP 的 hover 文案（「输入：`\and`（别名 `\wedge`）」）从 Rust 表渲染；这份 JS 表
// 只服务 VS Code 的缩写改写器（`abbreviation-rewriter.js`）。两边的
// symbol / abbreviation / aliases / supported / notationSymbol / **顺序** 由
// `crates/cli/tests/extension.rs::abbreviation_table_mirrors_the_single_source`
// 用 serde_json **真解析**后逐条比对（与
// `tm_grammar_keywords_follow_the_single_source` 同一形制）——改一边不改另一边，
// `cargo test -p sokonanoda-cli --test extension` 直接红。
//
// 为什么表是「一条一行、键带引号」的**纯 JSON 数组字面量**：Rust 侧要能真解析它，
// 不做 grep 掩膜（硬规则 4：判定不许靠文本比对）。**格式即契约，别重排。**
//
// `notationSymbol` 在 JS 侧**今天不消费**（改写器只认 symbol/abbreviation/aliases），
// 但它**进镜像**：契约测试要的是**逐条相等**，投影掉一列就等于给漂移留了缝
// （与 `supported` 同款——`supported` 也只被 `BY_ABBREVIATION` 那一层用）。
// 它是「这个符号是记法符号还是标识符」的标记：`α` 是**标识符**，把它的缩写敲成
// `α` 是对的，但**不能**把它当成 `∈` 那样的符号去喂词法（那是 Rust 侧的事）。
//
// **匿名构造子括号** `⟨`/`⟩`（D6）**是全表唯一的非字母缩写**（Lean 的 `\<` / `\>`，
// 别名键就是 `<`/`>` 两个字符）——数组体里**不许写注释**（格式即契约），说明写在这里。
//
// `''`（像）**没有条目**——与 Lean 4 一致（直接打两个单引号），设计 §5。
"use strict";

const TABLE = [
  { "symbol": "∧", "abbreviation": "and", "aliases": ["an", "wedge"], "supported": true, "notationSymbol": true },
  { "symbol": "∨", "abbreviation": "or", "aliases": ["vee"], "supported": true, "notationSymbol": true },
  { "symbol": "↔", "abbreviation": "iff", "aliases": ["leftrightarrow", "lr"], "supported": true, "notationSymbol": true },
  { "symbol": "¬", "abbreviation": "not", "aliases": ["neg", "lnot"], "supported": true, "notationSymbol": true },
  { "symbol": "→", "abbreviation": "to", "aliases": ["imp", "rightarrow"], "supported": true, "notationSymbol": true },
  { "symbol": "∀", "abbreviation": "forall", "aliases": ["all"], "supported": true, "notationSymbol": true },
  { "symbol": "∃", "abbreviation": "exists", "aliases": ["ex"], "supported": true, "notationSymbol": true },
  { "symbol": "≠", "abbreviation": "ne", "aliases": ["neq", "eqn"], "supported": true, "notationSymbol": true },
  { "symbol": "∈", "abbreviation": "in", "aliases": ["member", "mem"], "supported": true, "notationSymbol": true },
  { "symbol": "⊆", "abbreviation": "sub", "aliases": ["ss", "subseteqq", "subseteq", "subset"], "supported": true, "notationSymbol": true },
  { "symbol": "∪", "abbreviation": "cup", "aliases": ["union", "un"], "supported": true, "notationSymbol": true },
  { "symbol": "∩", "abbreviation": "cap", "aliases": ["inter", "intersection"], "supported": true, "notationSymbol": true },
  { "symbol": "\\", "abbreviation": "setminus", "aliases": [], "supported": true, "notationSymbol": true },
  { "symbol": "∅", "abbreviation": "empty", "aliases": ["emptyset", "varnothing"], "supported": true, "notationSymbol": true },
  { "symbol": "𝒫", "abbreviation": "powerset", "aliases": ["McP"], "supported": true, "notationSymbol": true },
  { "symbol": "ᶜ", "abbreviation": "compl", "aliases": ["complement"], "supported": true, "notationSymbol": true },
  { "symbol": "⁻¹'", "abbreviation": "preim", "aliases": ["preimage"], "supported": true, "notationSymbol": true },
  { "symbol": "×ˢ", "abbreviation": "xs", "aliases": [], "supported": true, "notationSymbol": true },
  { "symbol": "α", "abbreviation": "alpha", "aliases": ["a"], "supported": true, "notationSymbol": false },
  { "symbol": "β", "abbreviation": "beta", "aliases": ["b", "be"], "supported": true, "notationSymbol": false },
  { "symbol": "γ", "abbreviation": "gamma", "aliases": ["g", "ga"], "supported": true, "notationSymbol": false },
  { "symbol": "δ", "abbreviation": "delta", "aliases": ["de"], "supported": true, "notationSymbol": false },
  { "symbol": "ε", "abbreviation": "epsilon", "aliases": ["e", "ep", "eps"], "supported": true, "notationSymbol": false },
  { "symbol": "ζ", "abbreviation": "zeta", "aliases": ["ze"], "supported": true, "notationSymbol": false },
  { "symbol": "η", "abbreviation": "eta", "aliases": ["et"], "supported": true, "notationSymbol": false },
  { "symbol": "θ", "abbreviation": "theta", "aliases": ["th"], "supported": true, "notationSymbol": false },
  { "symbol": "ι", "abbreviation": "iota", "aliases": ["io"], "supported": true, "notationSymbol": false },
  { "symbol": "κ", "abbreviation": "kappa", "aliases": ["ka"], "supported": true, "notationSymbol": false },
  { "symbol": "λ", "abbreviation": "lambda", "aliases": ["la", "lamda", "lam", "fun"], "supported": true, "notationSymbol": false },
  { "symbol": "μ", "abbreviation": "mu", "aliases": ["m"], "supported": true, "notationSymbol": false },
  { "symbol": "ν", "abbreviation": "nu", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "ξ", "abbreviation": "xi", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "ο", "abbreviation": "omicron", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "π", "abbreviation": "pi", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "ρ", "abbreviation": "rho", "aliases": ["rh"], "supported": true, "notationSymbol": false },
  { "symbol": "σ", "abbreviation": "sigma", "aliases": ["si"], "supported": true, "notationSymbol": false },
  { "symbol": "τ", "abbreviation": "tau", "aliases": ["ta"], "supported": true, "notationSymbol": false },
  { "symbol": "υ", "abbreviation": "upsilon", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "φ", "abbreviation": "phi", "aliases": ["ph", "straightphi"], "supported": true, "notationSymbol": false },
  { "symbol": "χ", "abbreviation": "chi", "aliases": ["c", "ch"], "supported": true, "notationSymbol": false },
  { "symbol": "ψ", "abbreviation": "psi", "aliases": ["ps"], "supported": true, "notationSymbol": false },
  { "symbol": "ω", "abbreviation": "omega", "aliases": ["om"], "supported": true, "notationSymbol": false },
  { "symbol": "Α", "abbreviation": "Alpha", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Β", "abbreviation": "Beta", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Γ", "abbreviation": "Gamma", "aliases": ["G"], "supported": true, "notationSymbol": false },
  { "symbol": "Δ", "abbreviation": "Delta", "aliases": ["D"], "supported": true, "notationSymbol": false },
  { "symbol": "Ε", "abbreviation": "Epsilon", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ζ", "abbreviation": "Zeta", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Η", "abbreviation": "Eta", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Θ", "abbreviation": "Theta", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ι", "abbreviation": "Iota", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Κ", "abbreviation": "Kappa", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Λ", "abbreviation": "Lambda", "aliases": ["L", "Lamda"], "supported": true, "notationSymbol": false },
  { "symbol": "Μ", "abbreviation": "Mu", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ν", "abbreviation": "Nu", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ξ", "abbreviation": "Xi", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ο", "abbreviation": "Omicron", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Π", "abbreviation": "Pi", "aliases": ["p", "P"], "supported": true, "notationSymbol": false },
  { "symbol": "Ρ", "abbreviation": "Rho", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Σ", "abbreviation": "Sigma", "aliases": ["S"], "supported": true, "notationSymbol": false },
  { "symbol": "Τ", "abbreviation": "Tau", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Υ", "abbreviation": "Upsilon", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Φ", "abbreviation": "Phi", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Χ", "abbreviation": "Chi", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ψ", "abbreviation": "Psi", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "Ω", "abbreviation": "Omega", "aliases": [], "supported": true, "notationSymbol": false },
  { "symbol": "≈", "abbreviation": "approx", "aliases": ["thickapprox"], "supported": true, "notationSymbol": true },
  { "symbol": "∘", "abbreviation": "comp", "aliases": ["circ"], "supported": true, "notationSymbol": true },
  { "symbol": "⁻¹", "abbreviation": "inv", "aliases": ["sy"], "supported": true, "notationSymbol": true },
  { "symbol": "•", "abbreviation": "smul", "aliases": ["bub", "bu"], "supported": true, "notationSymbol": true },
  { "symbol": "⊕", "abbreviation": "oplus", "aliases": [], "supported": true, "notationSymbol": true },
  { "symbol": "⋃₀", "abbreviation": "sUnion", "aliases": [], "supported": true, "notationSymbol": true },
  { "symbol": "⋂₀", "abbreviation": "sInter", "aliases": [], "supported": true, "notationSymbol": true },
  { "symbol": "⟨", "abbreviation": "langle", "aliases": ["<"], "supported": true, "notationSymbol": false },
  { "symbol": "⟩", "abbreviation": "rangle", "aliases": [">"], "supported": true, "notationSymbol": false },
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
