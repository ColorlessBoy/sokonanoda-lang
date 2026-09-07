//! `sokonanoda watch <file>`：常驻监控教学画布，产出带版本号的 JSON Lines 事件流。
//!
//! 这是 L1 服务层的 CLI 形态（也是 agent 通道的直连形态）：
//! 文件每次变化都会以 `file.changed` 事件开场，随后是本轮 delta
//! （exercise.opened/solved/failed、decl.checked/failed）与诊断。

use crate::json_report::{print_json_line, span_json};
use sokonanoda_front::compile::CompileOptions;
use sokonanoda_front::session::Session;
use std::time::Duration;

pub(crate) fn watch(path: &str) -> std::process::ExitCode {
    let mut session = Session::new(CompileOptions::default());
    let mut last_content: Option<String> = None;
    let mut version: u64 = 0;
    println!("sokonanoda watch — monitoring {path} (Ctrl-C to exit)");
    loop {
        match std::fs::read_to_string(path) {
            Ok(src) => {
                if last_content.as_deref() == Some(src.as_str()) {
                    std::thread::sleep(Duration::from_millis(300));
                    continue;
                }
                last_content = Some(src.clone());
                version += 1;
                let update = session.update(&src, version);
                print_json_line(&serde_json::json!({
                    "type": "file.changed",
                    "version": version,
                    "recompiled_from": update.recompiled_from,
                }));
                for event in &update.delta {
                    print_json_line(&serde_json::json!({
                        "type": event.kind.code(),
                        "name": event.name,
                        "version": event.version,
                    }));
                }
                if let Some(diag) = &update.parse_error {
                    print_json_line(&serde_json::json!({
                        "type": "diagnostic",
                        "stage": diag.stage_code(),
                        "code": diag.code(),
                        "message": diag.message,
                        "hint": diag.hint(),
                        "span": span_json(diag.span),
                        "version": version,
                    }));
                }
                for err in &update.report.errors {
                    print_json_line(&serde_json::json!({
                        "type": "diagnostic",
                        "stage": err.stage().code(),
                        "code": err.code(),
                        "message": err.message,
                        "hint": err.hint(),
                        "span": span_json(err.span),
                        "version": version,
                    }));
                }
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            Err(_) => {
                // 文件可能正在被编辑器原子替换；稍等再试。
                std::thread::sleep(Duration::from_millis(300));
            }
        }
    }
}
