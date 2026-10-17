# 14 — 迭代实机验收

**What to build:** 迭代二全部 ticket 的实机收口：dev 实跑逐票验收、涉键击注入场景留 verify 脚本、安装包冒烟、tracker 收口（done / wontfix）、必要时补 CONTEXT.md 域词。

**Blocked by:** 01–13（全部前置 ticket）

**Status:** done（2026-10-17 迭代收口完成）

- [x] 01–13 逐票实机验收通过
- [x] verify 脚本留档 .scratch/youmi-iter2/verify/
- [x] tauri build 安装包冒烟
- [x] CONTEXT.md 域词按需补充（如"翻译"工作台工具词条、快捷键作用域）
- [x] 全部 ticket 状态收口

## Comments

### 2026-10-16 — 逐票实机验收

tickets 01–07、09–13 经 dev 实跑逐票验收通过（CDP 驱动 + 截图 + 人工清单，记录见各
ticket 与 verify/README.md）；08 首轮验收反馈 3 处呈现问题当日修复并截图复核。

### 2026-10-17 — 迭代收口

- **08 收口**：修复实机复核无回归，08 → done；01–13 全部 done，无 wontfix。
- **verify 留档**：verify/README.md 脚本表 07~13 行收口为"实机验收通过"，并加收口记录。
- **安装包冒烟**：`pnpm tauri build` 产出 MSI（Youmi_0.1.0_x64_en-US.msi，6.83 MiB）与
  NSIS（Youmi_0.1.0_x64-setup.exe，5.49 MiB）；release 产物 youmi.exe 实机启动正常
  （托盘常驻、10 秒存活后干净退出，验证前临时停掉的遗留 dev 实例未恢复）；MSI 管理
  解包校验 payload youmi.exe 与构建产物字节数一致。tsc 随 beforeBuildCommand 通过，
  vitest 12 文件 118 例全绿。
- **域词**：CONTEXT.md 补「翻译」（工具域）与「快捷键作用域」（应用结构）两词条。
