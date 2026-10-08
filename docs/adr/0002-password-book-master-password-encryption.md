# 密码本采用主密码加密，而非 Windows DPAPI

DPAPI 是 Windows 上的显然选择（免密、系统级保护），但它把密文绑定到当前 Windows 账户，换机或重装系统即失效，且与 ADR-0001 的未来 macOS 迁移路径冲突。决策：密码本采用主密码 + Argon2id 派生密钥 + AES-256-GCM，密文存 SQLite；解锁会话只把明文驻留在内存，超时自动锁定。

## Consequences

- 每次使用需要解锁一步，换取备份可跨机器、跨平台迁移
- DPAPI 日后可作为可选的便利模式重新提出，但不是默认
