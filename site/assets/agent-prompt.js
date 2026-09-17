// The one installation prompt shared by every page that offers a copy button
// (index.html, get-started.html, agents.html, en/index.html). Prompt text lives
// here only, so there is a single place to update — pages must not inline their
// own copy. Zero build: plain ES5, no dependencies.
//
// Usage:
//   <button type="button" data-copy-agent-prompt>复制安装 prompt</button>
//   <pre data-agent-prompt></pre>   <!-- filled with the prompt text -->
(function () {
  var TEXT = {
    zh:
      "请把 sokonanoda 安装为我的 Lean 4 证明老师：\n" +
      "\n" +
      "1. 克隆仓库并进入目录：\n" +
      "   git clone https://github.com/ColorlessBoy/sokonanoda-lang\n" +
      "   cd sokonanoda-lang\n" +
      "\n" +
      "2. 读 AGENTS.md。按它的 Setup 准备环境（仓库根执行，harness 中立、零 cargo）：先跑 scripts/soko setup，再跑 scripts/soko doctor --json。版本按仓库 Cargo.toml 锁定——禁用 releases/latest，不用 cargo，不碰官方 Lean 工具链。\n" +
      "\n" +
      "3. 按 skills/sokonanoda-teacher/SKILL.md 当老师：在 playground.sokonanoda 上出带 sorry 的练习，练习前挂 2-3 行 \"-- soko:hint ...\" 提示阶梯。（DeepSeek Harness 里仓库的 .agents/skills/ 会被自动发现，直接输入 /sokonanoda-teacher 即可；opencode 里技能与 /sokonanoda/* 命令同样自动可用。）\n" +
      "\n" +
      "4. 判卷只认内核：跑 scripts/soko grade playground.sokonanoda --json，逐行读 JSON 事件（禁止文本比对）。问「某处还差什么」就用 scripts/soko query state --file playground.sokonanoda --line <行> --col <列>（单 JSON 对象），别自己扫事件流。讲解与反馈用简体中文。\n" +
      "\n" +
      "就绪后先跑一次判卷，告诉我当前状态，再按我的进度推进下一步。",
    en:
      "Install sokonanoda as my Lean-4-style proof tutor:\n" +
      "\n" +
      "1. Clone the repository and enter it:\n" +
      "   git clone https://github.com/ColorlessBoy/sokonanoda-lang\n" +
      "   cd sokonanoda-lang\n" +
      "\n" +
      "2. Read AGENTS.md. Prepare the environment as its Setup section says (from the repository root, harness-neutral, no cargo): run scripts/soko setup, then scripts/soko doctor --json. Versions are pinned to the repository Cargo.toml — never releases/latest, no cargo, no official Lean toolchain.\n" +
      "\n" +
      "3. Follow skills/sokonanoda-teacher/SKILL.md and teach me: add sorry exercises to playground.sokonanoda, each preceded by a 2-3 line \"-- soko:hint ...\" ladder. (DeepSeek Harness discovers the repository's .agents/skills/ automatically — type /sokonanoda-teacher; opencode loads the same skills and the /sokonanoda/* commands with no setup either.)\n" +
      "\n" +
      "4. Grade only through the kernel: run scripts/soko grade playground.sokonanoda --json and read the JSON events line by line (never compare text). When you need to know what is still missing at this position, use scripts/soko query state --file playground.sokonanoda --line <line> --col <col> (one JSON object) instead of scanning the event stream. Explain in English.\n" +
      "\n" +
      "Once ready, run a grading pass first, tell me where I stand, then advance at my pace.",
  };
  var LABEL = {
    zh: { copy: "复制安装 prompt", copied: "已复制 ✓" },
    en: { copy: "Copy install prompt", copied: "Copied ✓" },
  };

  var lang = /^en\b/i.test(document.documentElement.lang || "") ? "en" : "zh";
  var text = TEXT[lang];

  var blocks = document.querySelectorAll("[data-agent-prompt]");
  for (var i = 0; i < blocks.length; i++) {
    blocks[i].textContent = text;
  }

  // clipboard API needs a secure context; fall back to execCommand so the
  // button still works when the page is opened from disk / plain http.
  function copyText(done) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done, function () {
        legacyCopy(done);
      });
      return;
    }
    legacyCopy(done);
  }

  function legacyCopy(done) {
    var area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    var ok = false;
    try {
      ok = document.execCommand("copy");
    } catch (err) {
      ok = false;
    }
    document.body.removeChild(area);
    if (ok) done();
  }

  var buttons = document.querySelectorAll("[data-copy-agent-prompt]");
  for (var j = 0; j < buttons.length; j++) {
    (function (btn) {
      btn.textContent = LABEL[lang].copy;
      btn.addEventListener("click", function () {
        copyText(function () {
          btn.textContent = LABEL[lang].copied;
          setTimeout(function () {
            btn.textContent = LABEL[lang].copy;
          }, 1600);
        });
      });
    })(buttons[j]);
  }
})();
