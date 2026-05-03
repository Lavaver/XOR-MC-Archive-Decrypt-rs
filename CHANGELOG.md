# 更新日志

## [4.2.3](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.2.3) - 2026-05-03

> [!CAUTION]
> 破坏性变更：由于仓库名称发生改变，该版本以前的所有版本的自动更新功能将无法正常使用。你可以继续使用之前的版本，但有可能不能获取到新版本的内容。<br>
> 如需更新，[在此处](https://github.com/Lavaver/Crypt-Dew-World/releases/latest) 手动下载一份最新版本的 Crypt Dew World 副本。

### 新增
- 新增 | 补齐软件更新的未本地化文本
> [!TIP]
> 这其实是从初版开始就一直老生常谈的小问题。**并不是每个字符串都要本地化，如果出现次数比较少就有可能不会去本地化，以减少我的工作量**。随着项目不断迭代，这些本地化小问题也会逐步得到改善。

- 新增 | 交互式自定义导出目录
> [!NOTE]
> 虽然在此之前本身具有 `-o / --output` 参数，但在未填写的情况下默认输出到来源目录的上一级（这是本项目一贯以来的默认行为。）<br>
> （“来源目录的上一级”简单点来说，假如你填写的目录是这样的 `C:\Users\example\AppData\Roaming\MinecraftPC_Netease_PB\minecraftWorlds\DE0+0L6QUL0=`，那么工具就会导出到 `C:\Users\example\AppData\Roaming\MinecraftPC_Netease_PB\minecraftWorlds` 这个目录中）<br>
> 本次更新将为对命令行参数不熟悉的小白提供交互式选择，如果你填写了 `-o / --output` 参数，那么其还会按照一贯的方法导出。

### 变更
- 变更 | 本地化文本措辞优化
- 修复 | 除“仅复制”模式外其他打包模式在完成打包后未清理解密后临时世界文件夹的问题

## [4.2.2](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.2.2) - 2026-05-03

### 新增
- 新增 | 区块状态概览
- 新增 | `--details` 参数，用于在处理过程中查阅详情
> [!NOTE]
> 这会禁用进度条与校验信息。不过对于想要查阅详细信息的人来说有进度条反而起反作用。禁用校验信息是刻意的。

## [4.2.1](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.2.1) - 2026-04-30

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

## [4.2.0](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.2.0) - 2026-04-28

### 新增
- 新增 | 新增密码学相关目录模块（`cryptography/`）。

### 变更
- 移动 | 已将可从 `main.rs` 中分离的方法移至单独的文件中。
- 移动 | 相关辅助代码已移至 `utils/` 模块目录。

## [4.1.0](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.1.0) - 2026-04-27

### 新增
- 发布 | 发布了新的 Rust 版本