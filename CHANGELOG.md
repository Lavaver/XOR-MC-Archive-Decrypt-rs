# 更新日志

## [4.2.2](https://github.com/Lavaver/XOR-MC-Archive-Decrypt-rs/tree/v4.2.2) - 2026-05-03

### 新增
- 新增 | 区块状态概览
- 新增 | `--details` 参数，用于在处理过程中查阅详情
> [!NOTE]
> 这会禁用进度条与校验信息。不过对于想要查阅详细信息的人来说有进度条反而起反作用。禁用校验信息是刻意的。

## [4.2.1](https://github.com/Lavaver/XOR-MC-Archive-Decrypt-rs/tree/v4.2.1) - 2026-04-30

> [!TIP]
> 碎碎念：早点发布我五一早点家里蹲...

### 新增
- 新增 | 为 tar 归档打包失败的可能路径输出添加本地化文本。
- 新增 | 软件更新功能。
> [!NOTE]
> 目前只支持 Windows 版可以使用软件更新。这是因为目前而言**还没有对其他平台预编译与 Release 的计划**。我们针对该功能对不同平台施加了不同的条件编译，在编译 Windows 版以外的其他版本的时候**不会编译更新功能，检查更新所有平台仍可使用**。
- 新增 | EaseTrojan 算法
> [!NOTE]
> 该算法**适合资源中心二次加密的地图**。目前而言，该算法**处于实验性阶段**，且集成在主动解密内，不能保证该算法能一定能对此类地图解除加密。如果遇到了问题，欢迎[提出 Issue](https://github.com/Lavaver/XOR-MC-Archive-Decrypt-rs/issues)。

### 变更
- 修改 | tar 归档打包的行为。

### 移除
- 移除 | `log` Crate
- 移除 | `log4rs` Crate

## [4.2.0](https://github.com/Lavaver/XOR-MC-Archive-Decrypt-rs/tree/v4.2.0) - 2026-04-28

### 新增
- 新增 | 新增密码学相关目录模块（`cryptography/`）。

### 变更
- 移动 | 已将可从 `main.rs` 中分离的方法移至单独的文件中。
- 移动 | 相关辅助代码已移至 `utils/` 模块目录。

## [4.1.0](https://github.com/Lavaver/XOR-MC-Archive-Decrypt-rs/tree/v4.1.0) - 2026-04-27

### 新增
- 发布 | 发布了新的 Rust 版本