# 本卷撞到的缺口（发现端）

> **权威台账在语言仓：`docs/gaps/ledger.jsonl`**（含 `fixed_in` 状态）。这里只放
> "发现端"的原始材料：一条缺口一个文件 + 最小复现，随后收编进权威台账。

命名与字段照 `docs/gaps/README.md`（`id` / `kind` / `severity` / `repro` / `today` /
`expected_lean` / `workaround` / `blocks`）。写新条目时优先用：

```bash
python3 scripts/gap.py list --kind library     # 标准库欠账单独看
python3 scripts/gap.py next                    # 下一张工作单（含可粘贴给另一个 agent 的 prompt）
```

本卷目前撞到的（已在权威台账里）：**G-01**（开练习签名不校验）、**G-02**（构造子无命名空间）、
**G-03**（`Exists` 形状的归纳被拒）、**G-04**（无 notation）、**G-12**（相对路径模块根）、
**G-13**（`axiom` 不吃 binder）、**G-14**（单宇宙 binder）、**G-15**（内核错误 span）、
**L-01…L-05**（标准库欠账与分层教训）。
