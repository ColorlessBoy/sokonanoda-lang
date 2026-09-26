use super::*;
use crate::testutil::next_socket;

/// **P1 判据**（2026-09-26 用户需求）：一次编译必须**成对**报 `$/progress`
/// —— `WorkDoneProgress::Begin` … `WorkDoneProgress::End`。
///
/// 为什么这条值得一条判据：在此之前编辑器**一个信号都没有**（慢文件看起来像
/// "冻住了" ✗），而进度通知最常见的 bug 不是"不发"，是**只发 Begin 不发 End**
/// （客户端那把进度条永远转 ✗）。所以判据钉的是**成对**，不是"收到过 progress"。
///
/// **反向验证**：把 `compile_worker` 里那两次 `send_progress` 删掉 ⇒ 本判据红
/// （`next_socket` 会在等 `$/progress` 上超时 ✓）。
#[tokio::test]
async fn compiling_reports_a_paired_work_done_progress() {
    let src = "def id (\u{3b1} : Type) (a : \u{3b1}) : \u{3b1} := a\n";
    let (mut service, mut socket) = test_service();
    handshake(&mut service).await;
    did_open(&mut service, src).await;

    let mut began = false;
    // 上限只是"看得见的失败"（`next_socket` 自己会超时 panic ✓）：中间会夹
    // `publishDiagnostics` 之类的通知 ⇒ 必须**跳过**而不是当成失败 ✓。
    for _ in 0..64 {
        let msg = next_socket(&mut socket, "$/progress（P1）").await;
        if msg.method() != "$/progress" {
            continue;
        }
        let params: ProgressParams =
            serde_json::from_value(msg.params().cloned().unwrap_or(json!(null)))
                .expect("valid ProgressParams");
        match params.value {
            ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(_)) => began = true,
            ProgressParamsValue::WorkDone(WorkDoneProgress::End(_)) => {
                assert!(began, "必须先 `Begin` 再 `End`（成对 ✓），不许只发 `End`");
                return;
            }
            _ => {}
        }
    }
    panic!("一次编译必须成对报 `$/progress`（P1）：只见到 Begin={began} 没等到 End");
}
