# 上次token用完前我的问题
'axiom And.intro — 已通过内核检查' 这种都不如 'axiom And.intro : forall (a b : Prop), a -> b -> And a b'. 另外使用 'And.intro' 的时候, vscode 点击跳转不对, 'And' 和 'intro' 分别下划线高亮,都跳转到 'And.intro' 这一行. 我理解'And' 应该跳转到 'And' 的定义, 'intro' 应该跳转到 'And.intro' 的定义.
你的产品设计文档还不够全面,检查不够全面细致.
======
我停留在 '->' 的时候,我希望看到对应的表达式以及它的类型.比如:
axiom Or.inr : (a : Prop) -> (b : Prop) -> b -> Or a b
停留在第一个 '->' 显示 '(a : Prop) -> (b : Prop) -> b -> Or a b : <类型1>'
停留在第二个 '->' 显示 '(b : Prop) -> b -> Or a b : <类型2>'
停留在第三个 '->' 显示 'b -> Or a b : <类型3>'
======
另外我想去掉 '???' 全用 "sorry", 你把我提的几点整理成全新的计划,重新实现