# Windows-only 实现，但 OS 耦合能力经由平台 seam 隔离

用户计划未来可能换用 macOS，因此不为了纯 Windows 的开发速度把 OS 耦合代码（剪贴板监听、全局快捷键、划词取词、浮层窗口定位、凭据存储）散落进业务代码。决策：只实现 Windows 一种后端，但上述能力各自收敛到单一 seam（Rust 平台模块/接口）后面；不预先编写任何 macOS 实现。

## Considered Options

- 纯 Windows、无抽象 —— 被拒：换机时迁移成本过高
- 第一天就做双平台 —— 被拒：没有 macOS 设备可验证，属过度投资

## Consequences

- 未来移植 = 按 seam 逐个补第二实现；seam 总数有限（约五个），除此之外不为跨平台做任何前期投资
- 每个 seam 只有单一实现时，review 时看起来像"过度设计"，这是有意为之
