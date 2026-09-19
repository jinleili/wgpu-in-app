# wgpu #8518：Texture 初始化后续实验

后续已从最新 trunk 创建独立分支并重新收敛实现，见[独立 texture 分支与提交前验证](issue_8518_texture_ready_result.md)。本文保留旧分支实验数据，不作为新分支的性能或验收结论。

**结论：本机已找到有重复收益的 texture 优化方向。最终 Box 版本的 compute 编码中位数改善 3.0%～8.0%，render 改善 5.0%～7.2%。当前是限定覆盖的实验候选，尚不能宣称所有负载无回归，也没有实现完整 tracker folding。**

本次独立从 `f7c40b2cf96327b9d2dd2ffef73400528d2ec4f7` 开始，不包含 buffer B 原型。工作区 `/Users/lijinlei/Rust/forks/wgpu-issue-8518-texture`，分支 `codex/issue-8518-texture-init`。

日期：2026-09-19。配套文档：[总体方案](issue_8518_implementation_plan.md)、[buffer A/B 记录](issue_8518_phase_ab_result.md)。

本次任务的完成标准是：定位 texture 成本；验证有限快速路径与 bind group 摘要是否有收益；用初始化、discard 回归检查明确边界；依据实测决定是否值得继续全面 folding。实验有结果不代表通过合入验收。

对用户提供的维护者反馈，纹理初始化是值得独立分析的方向。之前 B 的结果只约束 buffer 原型，不再作为研究 texture 的前置门槛。维护者描述的是其基准；本机仍须独立验证。

基线 compute 的 8 秒 sample：主线程 run_subpass 分支有 3552 样本；State::flush_bindings 有 1687；其中 texture register 调用点 198（含 check_action 131），HAL set_bind_group 分支 1354。调用点采样不能解释为 RwLock 本身耗时，亦不能当作准确端到端时间占比。可见 texture 成本，但未复现占绑定成本大多数。

窄候选：原 RwLock 外封装 AtomicU8 摘要，三态为不可缓存、跟踪中、永久完成。只有 native 且资源 usage 不含 RENDER_ATTACHMENT / TRANSIENT_ATTACHMENT 的纹理可缓存。查询在原读锁内检查所有 mip/layer，确认没有任何未初始化范围才发布；这是惰性读侧发布，不改写路径。Relaxed 不发布普通内存，不承担 GPU 完成同步。失败和旧值只走原查询。Discard 入口断言不可缓存；原 command-buffer-local discard 修复保持。

第二候选在 bind group 层额外汇总：只有每个引用纹理都达到永久完成，才能省去该 group 后续纹理初始化遍历。含可 discard 纹理的 group 不能获得摘要。资源销毁、设备、binding/barrier 验证仍执行。这个摘要并非维护者提出的完整 usage-scope → command-buffer → device folding。

| 方案 | 命中后的初始化检查成本 | 可 discard 纹理 | 实验定位 |
| --- | --- | --- | --- |
| 单纹理完成摘要 | 仍逐 action 访问纹理并读摘要 | 保留原路径 | 单独测量锁与 mip 查询的可消除成本 |
| Bind group 完成摘要 | 该组一次摘要读取，省去逐纹理遍历 | 组内任一纹理可 discard 就不能获得组摘要 | 当前实现方向 |
| 完整 tracker folding | 按 scope、command buffer、device 合并局部状态 | 必须设计首用要求、出口状态及就地 clear | 尚未实现，不能用当前数字宣称其收益 |


完整 folding 至少需要保留每个子资源的首用初始化需求及结束状态，按实际提交顺序与 device 合并，并在 discard 后同一 command buffer 再次读取前插入 inline clear。当前 barrier tracker 的 TextureUsageScope 与 TextureTracker 分别已有 scope、start/end 状态，但 TextureUses::UNINITIALIZED 表达布局转换时可丢弃内容，并不直接等于惰性清零 tracker 的完整语义。不可只把 bool 搬进 tracker，也不能把 global init 查询简单推迟到 submission 后宣称省去了成本。


## 正确性边界与验证方法

- 非附件纹理的初始化范围只会缩小。队列写入、提交时清零、外部显式初始化仍使用原有写锁及提交时序；摘要不是 GPU 完成标志。
- 每 mip 的所有 array layer 都初始化后才能发布整个纹理完成；D3 仍以整个 mip 为跟踪单位。一个已经初始化的 view 不能证明整张纹理完成。
- 部分消费后丢弃 drain 时，是否完成以原 tracker 的实际范围为准。
- 可 discard 纹理、surface 和 invalid 对象禁用摘要，不引入可反复置位的无锁布尔状态。
- bind group 的资源列表不可变。只有所有纹理的单向摘要都完成时，group 才能永久跳过 texture init actions；该组内不存在需要匹配的本地 discard。非合格组仍逐 action 处理原 discard 修复。
- Texture copy、attachment、render bundle 的原路径继续经过统一 texture 查询；本次 bind group 汇总只改变 `flush_bindings_helper`，没有声称所有 bundle replay 工作都已汇总。

新增五项内部测试覆盖多 mip/layer、不可缓存纹理的 discard、空 tracker 与旧摘要、部分消费 drain、持写锁期间快速查询。新增 GPU 回归覆盖窄 view 与全 view、三轮绑定、编码后 queue 写入，以及具有 attachment 使用能力的纹理在反复采样后 discard 再采样。已有 zero_init 用例补充 D3、depth/stencil、部分 copy 和外部初始化。

## 完整 folding 的边界

维护者描述的 scope → command buffer → device 方案需要单独设计以下契约，本次没有实现：

1. scope 收集首次读取要求与隐式全覆盖写入，按 mip/layer 保持有效范围；storage 写不能视为自动完整覆盖。
2. command buffer 合并重复使用，同时区分入口初始化需求与出口状态；`write → read`、`read → write`、`discard → read` 的语义不同。
3. 同一 command buffer 内的 discard 后读取仍须就地清零，不能把清理统一放到 command buffer 开头。
4. device 合并按实际 submission 顺序执行，支持先编码读命令、后编码 discard 命令、最终反序提交；不能把编码时的全局状态永久缓存给可 discard 纹理。
5. 复用 tracker 的稠密数组、资源索引和 simple/complex 子资源表示前，要明确生命周期、引用保留、D3 slice、aspect 和 surface 的边界。

参考：[WebGPU 的 discard 语义](https://gpuweb.github.io/gpuweb/#enumdef-gpustoreop)、仓库 `wgpu-core/src/track/mod.rs` 的 tracker 合并契约，以及 `track/texture.rs` 的 `TextureUsageScope` / `TextureTracker`。规范原文快照保存在实验目录 `webgpu-spec.bs`。


## 性能实验

硬件为 Apple M1 Max / Metal。基线与候选从同一源码目录构建，使用相同 benchmark 源码、release/bench profile、thin LTO 和调试符号，并固定成不同可执行文件。计时关闭额外 Metal/backend 调试验证，保持 core API 验证；本任务的构建、测试和 profiler 不与计时并行。

每个候选五组独立进程，交替 A→B / B→A；每进程每子场景 32 次。表中是进程内平均值的五组中位数，变化为 `(候选/基线 - 1) × 100%`，负数表示耗时减少。编码与 submit 单独计时、GPU 等待不计入两者。合计先逐进程相加，再取中位数。Compute/render 在计时前完成一次初始化、提交及 GPU 等待；惰性摘要可能在首次计时迭代才缓存，未将该次迭代从结果中删除。

### 单纹理摘要

Compute 编码中位数改善 0.1%～2.7%，render 改善 0.2%～3.1%，多数子场景配对方向不一致，未达到主场景 5% 目标。不建议仅凭这批小幅差异采用单纹理摘要。

### 增加 bind group 摘要：初始 Vec 版本

| 场景 | 基线编码 ms | 候选编码 ms | 编码变化 | 编码+提交变化 |
| --- | ---: | ---: | ---: | ---: |
| compute Encoding (2 passes) | 27.693 | 25.130 | -9.3% | -9.0% |
| compute Encoding (4 passes) | 28.782 | 26.642 | -7.4% | -6.9% |
| compute Encoding (8 passes) | 28.494 | 26.686 | -6.3% | -6.2% |
| compute Encoding (2 threads) | 14.320 | 13.145 | -8.2% | -7.3% |
| compute Encoding (4 threads) | 8.307 | 7.675 | -7.6% | -6.8% |
| compute Encoding (8 threads) | 5.714 | 5.304 | -7.2% | -5.7% |
| render Encoding (1 passes) | 42.585 | 40.540 | -4.8% | -4.4% |
| render Encoding (2 passes) | 39.996 | 38.539 | -3.6% | -3.7% |
| render Encoding (4 passes) | 40.272 | 39.496 | -1.9% | -2.1% |
| render Encoding (1 threads) | 40.635 | 38.309 | -5.7% | -5.4% |
| render Encoding (2 threads) | 21.023 | 19.617 | -6.7% | -5.7% |
| render Encoding (4 threads) | 12.020 | 11.605 | -3.5% | -2.4% |

Compute 的 2/4/8 passes、4 threads 在五个配对中均改善；2/8 threads 各有一次退化。没有使用置信区间或宣称统计显著性。该候选在既有 compute 主场景取得超过 5% 的中位数改善，且编码加提交也改善，证据强于单纹理方案。所有数字均为 CPU 基准，不是 GPU 执行或真实应用帧率。

### Buffer-only 对照

复用先前提交中的 `Buffer Initialization` 基准，仅作为没有纹理的回归负载；没有带入 B 的 buffer 快速路径。初轮 initialized 8 threads 出现编码 +18.4%、编码+提交 +12.2%，五对编码变化为 −12.7%、+80.1%、+24.4%、+54.2%、−9.4%。存在显著进程间波动，不能直接认定为固定退化，也不能将其忽略。随后独立增加到每进程 128 次迭代，退化方向仍在不同子场景间变动，波动没有消失，不能证明没有回归。复测首个配对曾与一次格式检查并行，已将该配对两边原始文件移入 `ab-buffer-recheck-rejected-format/`，再用相同二进制完整重跑该配对；汇总使用重跑数据。


### 最终 Box 版本

以下为当前保留源码的独立五组结果：compute/render 每进程 32 次，buffer 每进程 128 次。表中变化仍是耗时变化，负数表示更快。

| 场景 | 基线编码 ms | 候选编码 ms | 编码变化 | 编码+提交变化 |
| --- | ---: | ---: | ---: | ---: |
| compute Encoding (2 passes) | 28.425 | 26.306 | -7.5% | -7.2% |
| compute Encoding (4 passes) | 28.901 | 28.024 | -3.0% | -2.2% |
| compute Encoding (8 passes) | 29.187 | 26.857 | -8.0% | -7.2% |
| compute Encoding (2 threads) | 14.567 | 13.930 | -4.4% | -3.4% |
| compute Encoding (4 threads) | 8.702 | 8.247 | -5.2% | -3.0% |
| compute Encoding (8 threads) | 6.835 | 6.435 | -5.9% | -3.5% |
| render Encoding (1 passes) | 44.430 | 42.067 | -5.3% | -5.6% |
| render Encoding (2 passes) | 41.905 | 39.820 | -5.0% | -4.0% |
| render Encoding (4 passes) | 42.720 | 40.202 | -5.9% | -5.8% |
| render Encoding (1 threads) | 42.102 | 39.083 | -7.2% | -7.2% |
| render Encoding (2 threads) | 22.022 | 20.614 | -6.4% | -6.1% |
| render Encoding (4 threads) | 12.759 | 11.994 | -6.0% | -6.0% |
| buffer initialized: encoding (1 threads) | 1.723 | 1.670 | -3.0% | -1.9% |
| buffer initialized: encoding (2 threads) | 1.256 | 1.225 | -2.4% | +0.3% |
| buffer initialized: encoding (4 threads) | 0.830 | 0.798 | -3.9% | -3.5% |
| buffer initialized: encoding (8 threads) | 0.650 | 0.658 | +1.3% | +2.4% |
| buffer partial: encoding (1 threads) | 1.737 | 1.780 | +2.5% | +2.2% |
| buffer partial: encoding (2 threads) | 1.194 | 1.180 | -1.2% | -0.3% |
| buffer partial: encoding (4 threads) | 0.799 | 0.794 | -0.7% | -0.3% |
| buffer partial: encoding (8 threads) | 0.711 | 0.749 | +5.4% | -0.6% |
| buffer shared: encoding (1 threads) | 0.961 | 0.936 | -2.5% | -1.6% |
| buffer shared: encoding (2 threads) | 1.192 | 1.170 | -1.9% | -2.5% |
| buffer shared: encoding (4 threads) | 1.198 | 1.166 | -2.7% | -2.0% |
| buffer shared: encoding (8 threads) | 1.257 | 1.093 | -13.0% | -11.8% |
| buffer cold: encoding | 1.826 | 1.871 | +2.5% | +1.3% |
| buffer write: encoding | 1.926 | 1.928 | +0.1% | -0.8% |

Compute 的 2/8 passes、2/4 threads 五次配对均改善；4 passes 有一次 +0.3%。8 threads 波动明显，五对为 −10.8%、+8.2%、−5.9%、−17.8%、+53.4%，不能仅凭其中位数 −5.9% 宣称该子场景稳定改善。Render 六个子场景的五次配对均改善。

Buffer-only 的 initialized 8 threads 编码中位数最终为 +1.3%，编码加提交 +2.4%，没有复现初始 Vec 版本的固定 +18.4% 幅度。Partial 8 threads 编码中位数仍为 +5.4%，但编码加提交为 −0.6%，五对编码变化为 −18.7%、+10.2%、+7.2%、−9.8%、−8.2%。这些相互变化的信号说明高并发小负载仍不稳定，不能把复测写成“已证明无回归”。没有定位到确定根因，也没有把对象布局或系统调度的猜测写成事实。

最终判断：相对于只优化 buffer，值得独立推进 texture；相对于只加单纹理标志，当前数据更支持在 bind group 层减少重复纹理访问。2/8-pass compute 与 render 提供了较明确的本机收益证据。全面合入验收尚未完成：需要稳定高并发回归基线，并补充部分 mip、可 discard 纹理、资源创建敏感负载及其他后端的针对性验证。当前不据此关闭 #8518，也不把完整 folding 列为已交付。


## 验证记录

- `cargo fmt --all --check`、`git diff --check`、`cargo clippy --tests`、`cargo check -p wgpu-core --no-default-features` 通过。
- `cargo test -p wgpu-core --lib init_tracker`：16 项通过，包含新增 5 项 texture 测试；此分支不含 B 的 6 项 buffer 测试。
- 原生全量：1103 项运行，harness 总结 1100 pass、3 fail、6 skip。pass 包含 feature 不支持和预期失败用例，不能全部算成实际 GPU 执行。
- 新增 `zero_init::texture_init_completion` 实际执行通过。原生失败仍为两个缺少 `dxc` 的 passthrough 测试，以及 subgroup 测试预期失败但实际通过；三项均与先前在同一 f7c40b2cf 基线上复现的问题一致，没有修改预期或 skip 配置。

- 锁顺序校验：独立 target，`RUSTFLAGS="--cfg wgpu_validate_locks" cargo test -p wgpu-core --lib init_tracker`，16/16 通过。
- Texture 初始化 CTS：403 个顶层用例，349 pass、54 skip、0 fail；buffer 初始化 CTS：23/23 pass。二者均有有效 Summary。没有把本次针对性 CTS 扩展为“默认全量 CTS 已通过”。
- CTS revision 为 `83cbc838309ba7bddf6f3ec5c17574564773732b`，运行前核对与 `cts_runner/revision.txt` 一致。xtask 使用本 worktree 独立构建的运行文件，避免复用带有其他 checkout 路径的 runner。

## 内存成本与覆盖限制

在相同 `std + metal + wgsl` 配置测量：

| Core 对象 | 基线 | 初始 Vec 候选 | 最终 Box 候选 |
| --- | ---: | ---: | ---: |
| Buffer | 272 B | 272 B | 272 B |
| Texture | 680 B | 688 B | 688 B |
| BindGroup | 296 B | 304 B | 296 B |

Bind group 的 texture init actions 在创建后不再修改，因此最终改为 `Box<[TextureInitTrackerAction]>`，移除 Vec 的容量字段，为摘要抵消了 8B 成本。创建时 `into_boxed_slice` 可能收缩分配，其成本仍需在资源创建敏感负载中衡量。表中只是对象的 `size_of`，不代表分配器实际占用。没有混入 B 的 Buffer 大小变化。

运行验证只覆盖本机 Metal；Vulkan、DX12、GLES 和 Wasm 未做运行验证。`no-default-features` 编译不等于所有 no_std 目标已验证。Surface 明确排除；可 discard 纹理保持原检查，不宣称跨提交状态问题已经解决。真实应用、部分 mip 长期未初始化和大量可 discard 纹理的性能尚未专门测量。原生 readback 也不能替代 poison-memory 平台验证。

## 复现与产物

正式源码位于独立 worktree；现有 `codex/issue-8518-buffer-init` 和其提交没有改动。应用工程依赖路径未切换。本次尚未 commit、push 或创建 PR。

原始日志、固定二进制、源码差异快照和驱动脚本位于：

`/Users/lijinlei/Rust/forks/wgpu-issue-8518-texture/target/issue-8518-texture/`

- `ab-per-texture/`、`per-texture-bench`：只加单纹理摘要的五组对照。
- `ab/`、`group-vec-bench`：初始 Vec 版本 bind group 摘要的五组 compute/render/buffer 对照。
- `ab-box/`、`candidate-bench`：最终 Box 版本的五组对照，compute/render 每进程 32 次，buffer 每进程 128 次。
- `baseline-bench`：同一基准源码、未修改 core 的基线；可执行文件哈希见 `artifacts.json`。
- `ab-buffer-recheck/`：buffer 独立复测，原始首轮数据保留。
- `summary.json` / `summary.txt`：每轮原始数值、中位数、配对变化及编码加提交结果。
- `native.log`、`cts-texture.log`、`cts-buffer.log`、`unit.log`、`lock-validation.log`：初始 Vec 版本验证证据。
- `native-box.log`、`cts-texture-box.log`、`clippy-box.log`、`no-std-box.log`：最终 Box 版本的针对性验证证据。
- `candidate.patch`、`per-texture.patch`：阶段差异快照；额外复用的 `buffer_init.rs` 来自原 B 提交的 benchmark，作为无纹理回归负载。
- 基线 CPU 采样保存在原 B 工作区的 `target/issue-8518-texture/profile-compute.txt`，初始 Vec 版本的候选采样保存在本实验目录 `profile-candidate.txt`。在该时间窗内没有观察到 `register_init_action` / `TextureInitTracker::check_action` 样本；最终 Box 版本未另做采样。

代码验证命令：

```sh
cargo fmt --all --check
cargo clippy --tests
cargo test -p wgpu-core --lib init_tracker
cargo check -p wgpu-core --no-default-features
WGPU_BACKEND=metal cargo xtask test --no-fail-fast
cargo xtask cts --backend metal 'webgpu:api,operation,resource_init,texture_zero:*'
cargo xtask cts --backend metal 'webgpu:api,operation,resource_init,buffer:*'
```

最终五组对照使用 `AB_DIR=ab-box BUFFER_ITERS=128 python3 target/issue-8518-texture/run_ab.py`；初始 Vec 版本的 buffer 复测使用 `SCENARIO=buffer AB_DIR=ab-buffer-recheck BUFFER_ITERS=128 python3 target/issue-8518-texture/run_ab.py`。复现前须按当前源码构建两版并保存为上述不同的可执行文件，保持基准源码一致，且不与构建、GPU 测试、profiler 并行计时。脚本只运行固定二进制，不会自动重建。`target` 中的开发产物不纳入普通源码提交。


最终 Box 版本额外验证：`cargo clippy --tests` 通过；原生 zero_init 与 render bundle 筛选运行 51 项，harness 全部 pass（包含 feature 不支持的注册项，不能全部当作 GPU 实际执行）。此前全量 1103 项与锁顺序测试来自同一初始化逻辑的 Vec 版本；最终改动仅将创建后的 action 列表固定为不可变切片。

最终 Box 版本的 `no-default-features` 编译检查通过；texture 初始化 CTS 再次取得 349 pass、54 skip、0 fail 的有效总结，见 `no-std-box.log` 与 `cts-texture-box.log`。
