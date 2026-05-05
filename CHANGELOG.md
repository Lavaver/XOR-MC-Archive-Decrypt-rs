# 更新日志

## [4.3.0](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.3.0) - 2026-05-05

> [!CAUTION]
> **重要安全更新**：本次更新修复了**波及严重的路径与文件系统行为**。你应尽快动作，使用 `[程序名].exe -S` 对其进行软件更新。如果无法这么做，[在此处](https://github.com/Lavaver/Crypt-Dew-World/releases/latest) 手动下载一份最新版本的 Crypt Dew World 副本。

## 变更
- 修复 | 输入路径的一些玄学问题
> [!WARNING]
> 在此前版本中，文件夹路径会被当作打包基路径，如果不留意直接 `y` 选择覆盖，就会导致该文件夹下的东西都被丢失。现在的修复行为为：<br>
> - **仅输出模式下，当指定的输出目录是空的就照常将解密产物放在里面；**
> - **仅输出模式下，当指定的输出目录不是空的，则新建一个以 `名称_Dec` 为名的文件夹，然后在新的文件夹当中输出产物；**
> - **仅输出模式下，当指定的输出目录不存在，则以路径最后一个指定的目录名为名新建文件夹，然后在新的文件夹当中输出产物；**
> - **其余两个打包模式下都会在指定的输出目录新建以 `名称_Dec` 为名的 Tar 或 mcworld 归档，并在归档内输出产物**

## [4.2.4](https://github.com/Lavaver/Crypt-Dew-World/tree/v4.2.4) - 2026-05-04

### 变更
- 优化 | 交互式输入导出目录的提示变的更友好。在你未输入路径时，现在会以灰色字体显示默认情况下的导出目录。
> [!NOTE]
> 如果安装了国际版 Minecraft BE，且是仅复制模式，则默认情况下的导出目录将直接通往如下目录：
> ```text
> C:\Users\[你的用户名]\AppData\Roaming\Minecraft Bedrock\Users\[19 位用户 ID]\games\com.mojang\minecraftWorlds
> ```
> 届时直接打开国际版即可食用。如果你是早期 UWP 版的 Minecraft BE，你应该升级你的游戏，因为从 1.21 开始，Microsoft Studios 对游戏分发做出了改变，**即从原 UWP 版分发修改为 exe 版分发**，这同时也带来了存档存储路径的修改。**请注意：工具并不会去识别这是否为 UWP 还是 exe 版，因为要考虑到总体非平台特定性（您可能为了方便会在 Android 上构建代码然后在移动端处理加密），以及便携性（Windows Crate 带来的负担太重）**。

- 修复 | 输入选择操作在同一行的问题
- 优化 | 当输入请求提示符具有默认候选时会以 PlaceHolder 样式显示在提示符中

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