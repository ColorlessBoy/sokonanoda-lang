//! `sokonanoda fixtures [--json]` —— **判据的输入面**（生成式夹具）的唯一出口。
//!
//! ## 为什么 CLI 要出这个口（`AGENT-DOES-NOT-PRODUCE-USER-INPUTS`，2026-10-15，high）
//!
//! 用户的结构性诊断（原话见 `crates/front/src/fixtures.rs` 的模块文档）：
//! **"仪表够用，缺的是'用户会输入什么'—— Agent 从不喂那个输入"** ✗。
//!
//! 前两层已经接上同一个生成器（`crates/front/src/fixtures.rs`）：
//!
//! * 真相层 —— `crates/front/tests/process_state_fixtures.rs`（解析结构）；
//! * 入口层 —— `crates/cli/tests/process_state_entry.rs`（`query check` 的 counts）；
//! * **第三层（用户看到的东西）** —— `editor/vscode/src/test/extension.test.js`
//!   （真 VS Code + 真 LSP + 真扩展）。
//!
//! 第三层跑在 **Node** 里、被测的输入面却长在 **Rust** 里 ⇒ 中间必须有一座桥，
//! 否则 e2e 只能**手写一条实例** ✗ —— 而那正是这条 bug 的病根
//! （修 `SUB-parse-error-kills-whole-file` 时只补了两条手写用例，
//! 同类换个 tactic 就没人管 ✗）。
//!
//! ⇒ 本子命令把生成器的**全部**输出原样交出来（`--json`），e2e 侧
//! `execFileSync` 它即可 —— **一份输入面，三层共读** ✓（不是各写各的 ✗）。
//!
//! ## 契约
//!
//! ```text
//! sokonanoda fixtures           # 人读：家族分组 + id + 期望下界
//! sokonanoda fixtures --json    # 机读：**单个 JSON 对象**（与 `query` 同风格）
//! ```
//!
//! ```json
//! {"schema":"soko.fixtures/1","count":36,
//!  "families":[{"slug":"process-state","label":"过程态","count":24}, …],
//!  "inputs":[{"id":"…","family":"…","family_label":"…","why":"…","source":"…",
//!             "expect":{"declarations":2,"judged":2,"open":1,"examples":0}}]}
//! ```
//!
//! 退出码：`0` 交出来了 · `2` 用法错误（与 `docs/protocol.md` 的 0/1/2 契约一致）。
//!
//! ⚠ **本子命令不进判定路径**：它只**打印**生成器的输出，不碰 lexer/parser/judge，
//! 也不读任何 `.sokonanoda` 文件 ✓。

use std::process::ExitCode;

use sokonanoda_front::fixtures::{self, Family, UserInput};

/// 家族的 **ASCII 键**（`Family::name()` 是中文、给人看的；JSON 消费方要稳定的
/// 机器键 ⇒ 在这里映射，**不改 `crates/front`** ✓）。
fn family_slug(family: Family) -> &'static str {
    match family {
        Family::ProcessState => "process-state",
        Family::Differential => "differential",
        Family::Negative => "negative",
        Family::Boundary => "boundary",
    }
}

/// 四个家族在输出里的**固定顺序**（人读视图与 JSON 视图同一份顺序 ✓）。
const FAMILIES: [Family; 4] = [
    Family::ProcessState,
    Family::Differential,
    Family::Negative,
    Family::Boundary,
];

fn json_input(input: &UserInput) -> serde_json::Value {
    serde_json::json!({
        "id": input.id,
        "family": family_slug(input.family),
        "family_label": input.family.name(),
        "why": input.why,
        "source": input.source,
        "expect": {
            "declarations": input.expect.declarations,
            "judged": input.expect.judged,
            "open": input.expect.open,
            "examples": input.expect.examples,
        },
    })
}

/// `fixtures [--json]`：把 `front::fixtures::all()` 原样交出来。
pub fn fixtures_cmd(args: &[String], json: bool) -> ExitCode {
    // 本子命令没有位置参数（夹具不是"选出来的"，是生成器的全部输出 ✓）——
    // 多给参数是**用法错误**（exit 2），不是静默忽略 ✗。
    if let Some(extra) = args.first() {
        eprintln!("error: `fixtures` 不接受参数（给了 `{extra}`）");
        eprintln!("用法：sokonanoda fixtures [--json]");
        return ExitCode::from(2);
    }

    let all = fixtures::all();

    if json {
        let families: Vec<serde_json::Value> = FAMILIES
            .iter()
            .map(|family| {
                let count = all.iter().filter(|input| input.family == *family).count();
                serde_json::json!({
                    "slug": family_slug(*family),
                    "label": family.name(),
                    "count": count,
                })
            })
            .collect();
        let inputs: Vec<serde_json::Value> = all.iter().map(json_input).collect();
        let value = serde_json::json!({
            "schema": "soko.fixtures/1",
            "count": all.len(),
            "families": families,
            "inputs": inputs,
        });
        match serde_json::to_string_pretty(&value) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: 夹具序列化失败：{err}");
                ExitCode::FAILURE
            }
        }
    } else {
        println!(
            "sokonanoda fixtures —— 判据的输入面（生成式夹具，{} 份）",
            all.len()
        );
        println!("（机器可读：sokonanoda fixtures --json · schema soko.fixtures/1）");
        for family in FAMILIES {
            let members: Vec<&UserInput> =
                all.iter().filter(|input| input.family == family).collect();
            println!();
            println!("{}（{} 份）：", family.name(), members.len());
            for input in members {
                let expect = &input.expect;
                println!(
                    "  {}  ⇒ 声明 ≥{} · 判定 ≥{} · 未关目标 ≥{} · example ≥{}",
                    input.id, expect.declarations, expect.judged, expect.open, expect.examples
                );
            }
        }
        ExitCode::SUCCESS
    }
}
