# wgpu #8518：Windows 正确性验证交接

日期：2026-09-19。状态：代码已提交并推送，Windows 验证尚未执行。最新 Mac 实测和实现限制见 [render 流程复核](issue_8518_texture_render_result.md)。

- 代码仓库：[jinleili/wgpu](https://github.com/jinleili/wgpu)。
- 分支：`codex/issue-8518-texture-folding`。
- 提交：[`f47afd2b6aa8872eb283e808363e180f3930e9e3`](https://github.com/jinleili/wgpu/commit/f47afd2b6aa8872eb283e808363e180f3930e9e3)。
- 基线：`f12c3e4508cb706c1e1faeb973bdb9c6970f5e15`。

首次获取代码可执行：

```powershell
git clone --branch codex/issue-8518-texture-folding https://github.com/jinleili/wgpu.git wgpu-8518
cd wgpu-8518
git rev-parse HEAD
```

Poison-memory 不限定 Linux：当前基线的 Windows CI 也安装 Lavapipe，并继承全局 `LVP_POISON_MEMORY=true`。依据为该提交的 `.github/actions/install-mesa/action.yml` 与 `.github/workflows/ci.yml`。Windows Lavapipe 的通过可补足这种内存污染机制下的证据，但不能宣称 Linux 平台已实测。

下面整段可粘贴到 Windows 新对话中：

```text
请在这台 Windows 上完成 wgpu #8518 局部 texture initialization folding 的正确性验证，直接执行准备、测试和必要诊断，不只给操作建议。用中文汇报。

代码仓库：https://github.com/jinleili/wgpu
分支：codex/issue-8518-texture-folding
待验证提交：f47afd2b6aa8872eb283e808363e180f3930e9e3
基线提交：f12c3e4508cb706c1e1faeb973bdb9c6970f5e15
这是 wgpu 源码仓库，不是 wgpu-in-app，也不是旧 buffer 或永久缓存实验分支。

先读取 AGENTS.md、docs/testing.md，以及 .github/actions/install-mesa/action.yml 和相关 Windows CI 配置，核对 HEAD 与工作区。不要覆盖已有修改，不要擅自 rebase 或升级基线。根据实际 CPU 架构准备 Rust/MSVC、cargo-nextest、Vulkan SDK、DXC 和 Mesa Lavapipe；工具与原始日志优先放在 target/issue-8518-windows/。环境变量限定在测试进程，避免修改系统默认驱动或替换系统 DLL；需要我操作管理员安装时说明具体步骤，继续其余可执行工作。

本次实现是在 command buffer 内合并首次初始化需求，并跳过重复 bind group；相邻或重叠且并集仍为矩形的范围内联保存，复杂范围才分配 tracker。它不是完整 scope → command buffer → device folding。必须保留 discard 后就地清零、实际提交顺序、未提交命令不发布设备状态，以及资源销毁检查。

验证分三步：
1. Windows 原生 Vulkan + Lavapipe，设置 LVP_POISON_MEMORY=true，并用 VK_DRIVER_FILES 指定正确架构的 Lavapipe ICD。当前基线的 Windows CI 使用 Mesa 26.1.3 MSVC 包，可以参照但需核对本机架构与依赖。确认实际 adapter/driver 为 Lavapipe/llvmpipe，不是物理显卡或其他软件驱动；不能仅凭环境变量已设置就认定有效。
2. 先运行 CPU 范围合并测试、七个 texture_binding zero-init GPU 回归和四个 Texture Init Render 基准的测试模式，再运行其余 zero_init 测试。检查用例确实执行，不能把全跳过、没有 adapter 或返回码为零但没有运行测试视为通过。七项为 partial_view、dropped_command_buffer、discard_same_encoder、discard_separate_submissions、destroyed_texture、overlapping_views、reordered_submissions。
   可用起始过滤：cargo xtask test -E 'test(first_uses_preserve_order_and_gaps) | test(zero_init::texture_binding) | test(Texture Init Render)'
   补充零初始化：cargo xtask test -E 'binary(wgpu-gpu) & test(zero_init)'
   为证明测试能识别漏清零，在隔离副本中做一次有明确预期的定向临时故障注入，确认对应测试失败，再恢复并复测；故障代码和诊断探针不得进入交付源码。定向通过后，在候选上完成一次 Lavapipe poison-memory 的全量原生测试，报告每项失败和跳过。
3. 恢复驱动选择环境，用 Windows 真实显卡分别运行支持的 Vulkan、DX12 定向回归和四个 render 基准测试模式。重新生成并核对 .gpuconfig，避免继续误用 Lavapipe。缺少硬件支持的后端如实标为未验证；WARP 通过不能冒充真实显卡通过。

本机 Mac 上已完成：定向 58/58；格式、clippy、no_std、Wasm 编译检查通过；全量 1114 项运行，1111 通过、3 失败、6 跳过。三个 Mac 已知失败是两项 passthrough 缺 dxc，以及 subgroup_operations 预期失败但实际通过。Windows 出现失败时必须核对本机原因，不能直接套用这些结论。

本次重点是正确性，不重跑性能 A/B，不将软件驱动耗时当成性能验收。遵守 AGENTS.md：不在未经明确授权时运行 trunk 基线测试或 CTS；如果确实需要基线诊断，先列出具体最小对照范围。若确认候选有错误，做最小修复并运行必要检查，不扩大完整 folding 重构；不要自动提交、推送或创建 PR。

最终在 target/issue-8518-windows/ 保存报告、完整命令、工具与驱动版本、代码提交、adapter 信息、实际执行/通过/失败/跳过数量，以及故障注入前后结果。分别给出 Lavapipe poison-memory、硬件 Vulkan、硬件 DX12 的验证结论与剩余限制；正常路径最终源码必须不含临时故障或探针。
```
