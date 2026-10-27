// Guard for editor/vscode/language-configuration.json: the `·` bullet must
// indent the next line, exactly like Lean 4's language configuration
// (`leanprover.lean4-*/language-configuration.json` has the same onEnterRules).
//
// Why this test exists: the bullet feature landed without the editor-side
// `onEnterRules`, so pressing Enter after `  · intro h` kept the bullet's
// indentation instead of indenting the body. A static check is the cheapest
// layer that can see the missing rule; the real VS Code behaviour is covered by
// the e2e suite.
//
// Run: node editor/vscode/test-language-config.js

const assert = require("assert");
const fs = require("fs");
const path = require("path");

const file = path.join(__dirname, "language-configuration.json");
const config = JSON.parse(fs.readFileSync(file, "utf8"));

const rules = config.onEnterRules || [];
const rule = rules.find(
  (r) => r.action && r.action.indent === "indent" && typeof r.beforeText === "string" && r.beforeText.includes("·")
);
assert.ok(rule, "language-configuration.json must have Lean 4's `·` onEnter indent rule");

const re = new RegExp(rule.beforeText);
assert.ok(re.test("  · intro h"), "the bullet rule must match `  · intro h`");
assert.ok(re.test("  · "), "the bullet rule must match a bare `  · ` line");
assert.ok(re.test("  · constructor"), "the bullet rule must match `  · constructor`");
assert.ok(!re.test("  intro h"), "the bullet rule must not fire on `intro h`");
assert.ok(!re.test("  constructor"), "the bullet rule must not fire on `constructor`");
assert.ok(!re.test("  exact hx"), "the bullet rule must not fire on `exact hx`");

console.log("language-configuration: `·` Enter indent ✓ (Lean 4 onEnterRules)");
