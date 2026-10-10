# 19 — 全量 code review + 风格修正

**What to build:** 以 style guide（18）为 Standards 轴、以各 ticket 为 Spec 轴，对 MVP 规划完成点以来的全部代码提交做整体 review；风格类发现当场修正，包括 components 目录下 tsx 文件重命名为大驼峰（Windows 大小写不敏感文件系统上用 git 两段式改名保历史）。超出风格范围的结构性问题记录为新票，不顺手改。

**Blocked by:** 15 — Alt+V 热修；16 — design token 体系；17 — 自定义标题栏与双卡片布局；18 — style guide

**Status:** ready-for-agent

- [ ] review 报告产出：Standards + Spec 两轴发现并列
- [ ] 风格类发现全部修正，lint / fmt / 前后端测试全绿
- [ ] components 目录 tsx 均为大驼峰命名，导入更新完整、构建通过
- [ ] 超出风格范围的结构性发现已立票或记录
