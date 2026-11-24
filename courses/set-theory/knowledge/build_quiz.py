#!/usr/bin/env python3
"""把题库渲染成**零延迟自测页**：data/questions.json → out/quiz.html（单文件、离线可用）。

判卷在浏览器本地完成（数据里已有 answer/why），不需要模型参与 → 点选项立刻出对错。
模型只负责低频讲解与复盘。用法：

    python3 build_quiz.py
    open out/quiz.html                 # Mac 上直接打开
    # 手机上：同一 Wi-Fi 下访问 http://<mac-ip>:8765/out/quiz.html
"""
import json
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DATA = ROOT / "data" / "questions.json"
OUT = ROOT / "out"
TOKEN = "__BANK_JSON__"
BUILT = "__BUILT_AT__"

HTML = r"""<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>集合论 · 题库自测</title>
<style>
:root{--bg:#f2f4f8;--card:#fff;--ink:#11151c;--muted:#5b6472;--line:#e6e9ef;--soft:#f6f8fb;
  --accent:#2f6bff;--accent-soft:#eaf0ff;--ok:#12805c;--ok-soft:#e8f7f1;--bad:#c62f2f;--bad-soft:#fdecec}
@media (prefers-color-scheme:dark){:root{--bg:#0f1216;--card:#181c22;--ink:#eef2f8;--muted:#9aa3b2;--line:#2a3038;--soft:#1f242c;
  --accent:#5f97ff;--accent-soft:#17233c;--ok:#57d9a8;--ok-soft:#12291f;--bad:#ff8a8a;--bad-soft:#2b1717}}
*{box-sizing:border-box;margin:0;padding:0}
html,body{background:var(--bg);color:var(--ink)}
body{font:16px/1.55 -apple-system,BlinkMacSystemFont,"PingFang SC","Helvetica Neue",system-ui,sans-serif;
  padding:calc(14px + env(safe-area-inset-top)) 14px calc(20px + env(safe-area-inset-bottom));max-width:760px;margin:0 auto}
.head{display:flex;align-items:baseline;justify-content:space-between;gap:12px;margin-bottom:10px}
.title{font-size:19px;font-weight:750}
.badge{font-size:12px;color:var(--muted);white-space:nowrap}
.bar{height:7px;border-radius:99px;background:var(--soft);overflow:hidden;margin:0 0 16px}
.bar i{display:block;height:100%;width:0;background:var(--accent);border-radius:99px;transition:width .18s ease}
.panel{background:var(--card);border:1px solid var(--line);border-radius:18px;padding:18px;box-shadow:0 2px 10px rgba(15,23,42,.05)}
.start h2{font-size:20px;margin-bottom:6px}
.start p{color:var(--muted);font-size:14px;margin-bottom:14px}
.filters{display:flex;gap:10px;flex-wrap:wrap;margin-bottom:16px}
.filters label{flex:1;min-width:120px;font-size:12.5px;color:var(--muted);display:flex;flex-direction:column;gap:5px}
select,button{font:inherit;color:inherit}
select{appearance:none;-webkit-appearance:none;background:var(--soft);border:1px solid var(--line);border-radius:11px;
  padding:10px 12px;min-height:44px}
button{cursor:pointer;border:0;border-radius:12px;min-height:48px;padding:0 18px;font-weight:650;touch-action:manipulation}
.primary{background:var(--accent);color:#fff;width:100%}
.primary:disabled{opacity:.4;cursor:default}
.ghost{background:transparent;color:var(--muted);border:1px solid var(--line)}
.qmeta{display:flex;justify-content:space-between;align-items:center;font-size:12.5px;color:var(--muted);margin-bottom:8px;gap:10px}
.qmeta .tag{background:var(--accent-soft);color:var(--accent);border-radius:99px;padding:3px 10px;font-weight:650;white-space:nowrap}
.qmeta .pos{white-space:nowrap}
.qtext{font-size:18px;font-weight:700;line-height:1.5;margin:2px 0 14px}
.qtext code{font-family:Menlo,monospace;font-size:.88em;background:var(--soft);border-radius:6px;padding:1px 6px}
.opts{display:flex;flex-direction:column;gap:9px}
.opt{display:flex;align-items:center;gap:11px;width:100%;min-height:50px;padding:11px 13px;text-align:left;
  background:var(--card);border:1px solid var(--line);border-radius:13px;font-size:15.5px;line-height:1.45;cursor:pointer}
.opt .letter{flex:none;width:26px;height:26px;border-radius:8px;border:1px solid var(--line);background:var(--soft);
  display:inline-flex;align-items:center;justify-content:center;font-size:12.5px;font-weight:750;color:var(--muted)}
.opt .txt{flex:1;min-width:0}
.opt .txt code{font-family:Menlo,monospace;font-size:.88em;background:var(--soft);border-radius:6px;padding:1px 5px}
.opt.picked{border-color:var(--accent);box-shadow:0 0 0 2px var(--accent-soft)}
.opt.ok{border-color:var(--ok);background:var(--ok-soft)}
.opt.ok .letter{background:var(--ok);border-color:var(--ok);color:#fff}
.opt.bad{border-color:var(--bad);background:var(--bad-soft)}
.opt.bad .letter{background:var(--bad);border-color:var(--bad);color:#fff}
.opt:disabled{opacity:.92;cursor:default}
.why{margin-top:14px;border-radius:13px;padding:13px 15px;background:var(--soft);font-size:14px;line-height:1.6}
.why b{font-weight:750}
.why .src{display:block;margin-top:6px;font-size:12px;color:var(--muted)}
.actions{display:flex;gap:10px;margin-top:14px}
.actions .primary{flex:1;width:auto}
.actions .ghost{flex:none}
.result h2{font-size:22px;margin-bottom:4px}
.result .score{font-size:40px;font-weight:800;margin:6px 0 4px}
.result .score small{font-size:18px;color:var(--muted);font-weight:600}
.wrong{margin:14px 0;display:flex;flex-direction:column;gap:10px}
.wrong .item{border:1px solid var(--line);border-radius:13px;padding:12px 14px;background:var(--soft);font-size:14px;line-height:1.55}
.wrong .item code{font-family:Menlo,monospace;font-size:.88em;background:var(--card);border-radius:6px;padding:1px 5px}
.hint{font-size:12.5px;color:var(--muted);margin-top:10px}
.hidden{display:none!important}
@media(max-width:420px){.qtext{font-size:17px}.opt{font-size:15px}}
</style>
</head>
<body>
<div class="head">
  <div class="title">📚 集合论 · 题库自测</div>
  <div class="badge" id="built">__BUILT_AT__</div>
</div>
<div class="bar"><i id="bar"></i></div>

<section class="panel start" id="startView">
  <h2>选一组题，立刻开始</h2>
  <p>判卷在本地完成（题库已固化），点选项立刻出对错与解析，不用等老师。</p>
  <div class="filters">
    <label>章节<select id="chapter"></select></label>
    <label>难度<select id="level"></select></label>
    <label>题量<select id="count">
      <option value="5">5 题（热身）</option>
      <option value="10" selected>10 题</option>
      <option value="20">20 题</option>
      <option value="0">全部</option>
    </select></label>
  </div>
  <button class="primary" id="startBtn">开始答题</button>
  <p class="hint" id="poolHint"></p>
</section>

<section class="panel hidden" id="quizView">
  <div class="qmeta"><span class="pos" id="pos"></span><span class="tag" id="tag"></span></div>
  <div class="qtext" id="qtext"></div>
  <div class="opts" id="opts"></div>
  <div class="why hidden" id="why"></div>
  <div class="actions">
    <button class="primary hidden" id="nextBtn">下一题</button>
    <button class="ghost" id="quitBtn">退出</button>
  </div>
</section>

<section class="panel hidden" id="resultView">
  <h2>本轮完成</h2>
  <div class="score" id="score"></div>
  <div class="wrong" id="wrongList"></div>
  <div class="actions">
    <button class="primary" id="againBtn">再来一轮</button>
    <button class="ghost" id="copyBtn">复制成绩</button>
  </div>
  <p class="hint" id="copyHint"></p>
</section>

<script>
(function () {
  var BANK = __BANK_JSON__;
  var LETTERS = "ABCDEF";
  var state = { pool: [], index: 0, answers: [], correct: 0, current: null, locked: false, roundWrong: [] };
  var $ = function (id) { return document.getElementById(id); };

  function rich(text) {
    var parts = String(text == null ? "" : text).split("`");
    var out = "";
    for (var i = 0; i < parts.length; i += 1) {
      if (i % 2 === 1) out += "<code>" + escapeHtml(parts[i]) + "</code>";
      else out += escapeHtml(parts[i]);
    }
    return out;
  }
  function escapeHtml(text) {
    return String(text).replace(/[&<>"]/g, function (ch) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[ch];
    });
  }
  function shuffle(list) {
    var copy = list.slice();
    for (var i = copy.length - 1; i > 0; i -= 1) {
      var j = Math.floor(Math.random() * (i + 1));
      var t = copy[i]; copy[i] = copy[j]; copy[j] = t;
    }
    return copy;
  }
  function chapters() {
    var seen = {};
    BANK.questions.forEach(function (q) { seen[q.chapter] = true; });
    return Object.keys(seen).sort();
  }
  function fillFilters() {
    var chapterSelect = $("chapter");
    chapterSelect.innerHTML = '<option value="">全部章节</option>';
    chapters().forEach(function (chapter) {
      var option = document.createElement("option");
      option.value = chapter;
      option.textContent = chapter + " · " + (BANK.chapters[chapter] || "");
      chapterSelect.appendChild(option);
    });
    var levelSelect = $("level");
    levelSelect.innerHTML = '<option value="">全部难度</option>' +
      '<option value="1">L1 回忆</option><option value="2">L2 理解</option><option value="3">L3 辨析</option>';
    refreshPoolHint();
    chapterSelect.addEventListener("change", refreshPoolHint);
    levelSelect.addEventListener("change", refreshPoolHint);
  }
  function filtered() {
    var chapter = $("chapter").value;
    var level = $("level").value;
    return BANK.questions.filter(function (q) {
      return (chapter === "" || q.chapter === chapter) && (level === "" || String(q.level) === level);
    });
  }
  function refreshPoolHint() { $("poolHint").textContent = "当前可选：" + filtered().length + " 题"; }

  function start() {
    var pool = filtered();
    var count = Number($("count").value);
    if (pool.length === 0) { $("poolHint").textContent = "没有符合条件的题目"; return; }
    state.pool = shuffle(pool).slice(0, count === 0 ? pool.length : Math.min(count, pool.length));
    state.index = 0; state.answers = []; state.correct = 0; state.roundWrong = [];
    $("startView").classList.add("hidden");
    $("resultView").classList.add("hidden");
    $("quizView").classList.remove("hidden");
    render();
  }
  function render() {
    state.locked = false;
    $("nextBtn").classList.add("hidden");
    $("why").classList.add("hidden");
    $("why").innerHTML = "";
    var q = state.pool[state.index];
    state.current = q;
    $("pos").textContent = "第 " + (state.index + 1) + " / " + state.pool.length + " 题";
    $("tag").textContent = q.chapter + " · " + q.unit + " · L" + q.level;
    $("qtext").innerHTML = rich(q.prompt);
    $("bar").style.width = (state.index / state.pool.length) * 100 + "%";
    var opts = $("opts");
    opts.innerHTML = "";
    q.options.forEach(function (option, index) {
      var button = document.createElement("button");
      button.type = "button";
      button.className = "opt";
      button.innerHTML = '<span class="letter">' + LETTERS[index] + '</span><span class="txt">' + rich(option) + '</span>';
      button.addEventListener("click", function () { answer(index + 1, button); });
      opts.appendChild(button);
    });
  }
  function answer(choice, button) {
    if (state.locked) return;
    state.locked = true;
    var q = state.current;
    var ok = choice === q.answer;
    Array.prototype.forEach.call($("opts").querySelectorAll(".opt"), function (node, index) {
      node.disabled = true;
      if (index + 1 === q.answer) node.classList.add("ok");
    });
    if (!ok) { button.classList.add("bad"); state.roundWrong.push(q.id); }
    else { state.correct += 1; }
    state.answers.push({ id: q.id, choice: choice, correct: ok });
    var why = $("why");
    why.classList.remove("hidden");
    why.innerHTML = "<b>" + (ok ? "✅ 对" : "❌ 错，正确答案是 " + LETTERS[q.answer - 1]) + "</b>　" + rich(q.why) +
      '<span class="src">出处：' + escapeHtml(q.source) + "</span>";
    $("nextBtn").classList.remove("hidden");
    $("nextBtn").textContent = state.index + 1 < state.pool.length ? "下一题" : "看结果";
    $("bar").style.width = ((state.index + 1) / state.pool.length) * 100 + "%";
  }
  function next() {
    if (state.index + 1 < state.pool.length) { state.index += 1; render(); }
    else finish();
  }
  function finish() {
    $("quizView").classList.add("hidden");
    $("resultView").classList.remove("hidden");
    $("bar").style.width = "100%";
    $("score").innerHTML = state.correct + " / " + state.pool.length + " <small>正确</small>";
    var wrong = $("wrongList");
    wrong.innerHTML = "";
    state.answers.filter(function (a) { return !a.correct; }).forEach(function (a) {
      var q = state.pool.filter(function (item) { return item.id === a.id; })[0];
      var item = document.createElement("div");
      item.className = "item";
      item.innerHTML = "<b>" + q.id + "</b>　" + rich(q.prompt) +
        '<br>正确答案：' + LETTERS[q.answer - 1] + "　" + rich(q.options[q.answer - 1]) +
        "<br>" + rich(q.why);
      wrong.appendChild(item);
    });
    if (state.roundWrong.length === 0) wrong.innerHTML = "<div class='item'>全对，没有错题 🎉</div>";
    save();
  }
  function save() {
    try {
      var key = "soko.quiz.history.v1";
      var history = JSON.parse(window.localStorage.getItem(key) || "[]");
      history.unshift({ at: new Date().toISOString(), total: state.pool.length, correct: state.correct, wrong: state.roundWrong });
      window.localStorage.setItem(key, JSON.stringify(history.slice(0, 30)));
    } catch (error) { /* ignore */ }
  }
  function copyReport() {
    var lines = ["集合论题库自测 · 成绩单", "得分：" + state.correct + "/" + state.pool.length,
      "错题：" + (state.roundWrong.length ? state.roundWrong.join(", ") : "无")];
    state.answers.forEach(function (a) {
      var q = state.pool.filter(function (item) { return item.id === a.id; })[0];
      lines.push((a.correct ? "✅ " : "❌ ") + q.id + "（选 " + LETTERS[a.choice - 1] + "，正确 " + LETTERS[q.answer - 1] + "）");
    });
    var text = lines.join("\n");
    var done = function () { $("copyHint").textContent = "已复制，可以直接粘贴到聊天里发给老师。"; };
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done, function () { $("copyHint").textContent = "复制失败，手动选中复制。"; });
    } else {
      var area = document.createElement("textarea");
      area.value = text; document.body.appendChild(area); area.select();
      try { document.execCommand("copy"); done(); } catch (error) { $("copyHint").textContent = "复制失败。"; }
      area.remove();
    }
  }
  $("startBtn").addEventListener("click", start);
  $("nextBtn").addEventListener("click", next);
  $("quitBtn").addEventListener("click", function () {
    $("quizView").classList.add("hidden"); $("startView").classList.remove("hidden"); $("bar").style.width = "0";
  });
  $("againBtn").addEventListener("click", function () {
    $("resultView").classList.add("hidden"); $("startView").classList.remove("hidden"); $("bar").style.width = "0";
    refreshPoolHint();
  });
  $("copyBtn").addEventListener("click", copyReport);
  document.addEventListener("keydown", function (event) {
    if ($("quizView").classList.contains("hidden")) return;
    if (!state.locked && /^[1-6]$/.test(event.key)) {
      var node = $("opts").querySelectorAll(".opt")[Number(event.key) - 1];
      if (node) node.click();
    } else if (state.locked && (event.key === "Enter" || event.key === " ")) { next(); }
  });
  fillFilters();
  window.__QUIZ__ = { bank: BANK, state: state, start: start };
})();
</script>
</body>
</html>
"""

def main() -> int:
    bank = json.loads(DATA.read_text(encoding="utf-8"))
    html = HTML.replace(TOKEN, json.dumps(bank, ensure_ascii=False)).replace(
        BUILT, "生成于 " + datetime.now().strftime("%Y-%m-%d %H:%M"))
    OUT.mkdir(parents=True, exist_ok=True)
    target = OUT / "quiz.html"
    target.write_text(html, encoding="utf-8")
    print("QUIZ PAGE " + str(target) + "  " + str(target.stat().st_size // 1024) + " KB")
    print("questions embedded: " + str(len(bank["questions"])))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
