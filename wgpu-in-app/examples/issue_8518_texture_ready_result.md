# wgpu #8518：独立 texture 分支与提交前验证

日期：2026-09-19。本文记录从最新 trunk 重新收敛后的实现；[上一轮 texture 实验](issue_8518_texture_result.md)中的性能数字和单纹理状态设计不代表本分支。

后续已针对附件、混合组和共享纹理建立[command buffer 局部 folding 候选与三方对照](issue_8518_texture_folding_result.md)。该候选在独立分支中，本页继续保留永久完成缓存版本的实现和数据。

最新的真实 render 场景、范围合并修正和性能证据见[render 流程复核](issue_8518_texture_render_result.md)。永久缓存工作区保持不动。

## 分支与范围

- 基线：`f12c3e4508cb706c1e1faeb973bdb9c6970f5e15`，创建分支前通过 `git ls-remote` 确认 `gfx-rs/wgpu` 与 fork 的 `trunk` 均指向该提交，再 fetch fork。
- 分支：`codex/issue-8518-texture-ready`。
- 工作区：`/Users/lijinlei/Rust/forks/wgpu-issue-8518-texture-ready`。
- 原 buffer 实验与旧 texture 工作区保留。新分支没有 cherry-pick buffer 实验提交，没有改 buffer 初始化逻辑、`Texture` 结构或全局 texture tracker。
- 生产改动限于 `binding_model.rs`、`device/resource.rs`、`command/memory_init.rs`、`command/pass.rs`。其余为 GPU 回归、基准及基准说明。
- 当前交付为待提交工作区；本轮未执行 commit、push 或创建 PR。

本轮完成标准：从最新 trunk 独立实现；缩小生产改动与维护成本；补齐部分视图、discard、未提交命令和销毁验证；通过相关静态检查与原生测试；以同条件性能实验确认收益，并核对未命中缓存路径的开销。此标准不等同于所有后端 CI 已通过。

## 实现为何改变

旧原型先给每个 texture 增加全局完成标记，再给 bind group 增加摘要。它要求整张纹理的所有 mip/layer 均初始化，且增加了 Texture 对象体积。

本分支直接缓存 **bind group 的固定视图范围已完成初始化**。一个 view 的范围全部初始化后，即使未引用的 mip/layer 仍未初始化，该组也能命中缓存。全局 tracker 和写入路径保持原状。

创建 bind group 时只计算一次资格：每个 texture 都必须是 native，并且 usage 不含 `RENDER_ATTACHMENT`。合法 transient texture 必须同时包含 `RENDER_ATTACHMENT`，因此无需重复检查 `TRANSIENT_ATTACHMENT`。其中任意一个 texture 不满足，就让整个组继续走原有初始化及本地 discard 修复流程。Surface 不满足资格。没有 texture actions 的组直接完成。

首次使用合格组时，在原有读锁下逐 action 检查设备初始化状态，记录仍需初始化的范围。只有每个 action 都已不需要初始化，才把组内 `AtomicBool` 永久设为 true。后续 compute/render pass 绑定该组时，用一次标记读取代替重复访问各 texture 的锁和 mip/layer tracker。

texture actions 创建后不变，因此由 Vec 改为 Box slice，去掉容量字段，为两个状态字段腾出空间。创建时可能发生收缩分配，专用创建基准覆盖这一成本。

相同本机 bench 构建配置下，分别使用各版本编译出的 core 库测量，Buffer / Texture / BindGroup 在两边均为 **272 / 680 / 296 字节**。这里只统计 `size_of`，不代表堆分配器的实际占用。

## 正确性边界

1. 缓存的是设备 tracker 已确认的初始化状态，不是“已经编码了一条清零命令”。未提交或被丢弃的命令不能让标记完成。
2. 非附件 native texture 的初始化范围只会缩小；队列写入、提交清零与 `mark_externally_initialized` 仍使用原来的锁及顺序。
3. `Relaxed` 标记只决定是否省去冗余检查，不发布其他普通内存，也不是 GPU 完成信号。读到旧 false 只会多检查；该标记没有 true → false 转换。
4. 可 discard texture 的组仍调用原 `register_init_action`。同一 command buffer 的立即清零和提交后的全局 discard 状态均保持。
5. 缓存命中后仍执行资源有效性、提交时的销毁检查、usage/barrier tracking 和 HAL 绑定。新增测试实际验证销毁后提交仍报错。
6. Render bundle 仍保留完整 texture action 列表，回放路径没有改为使用本缓存。
7. 这是一项限定范围的 bind group 缓存优化，没有实现完整 usage-scope → command-buffer → device folding，也没有据此关闭 #8518。

规范依据：[texture 创建时零初始化](https://gpuweb.github.io/gpuweb/#dom-gpudevice-createtexture)、[discard 后的零值语义](https://gpuweb.github.io/gpuweb/#enumdef-gpustoreop)。本轮按仓库 `webgpu-specs` 技能重新下载并核对了规范。

## 正确性与构建验证

- `cargo clippy --all-targets --all-features` 通过。
- `RUSTFLAGS='' cargo check -p wgpu-core --no-default-features` 通过。
- `RUSTFLAGS='' cargo check -p wgpu-core --target wasm32-unknown-unknown --no-default-features` 通过。
- 定向 zero_init、bundle、resource_error 与新增 benchmark 测试：55 项通过。包含不支持 feature 的 harness 项目，不能全部算作实际 GPU 执行。
- 首轮新增 GPU 测试与新增 benchmark 测试：2 项通过；后续测试拆分和补强见下文。
- 首轮全量 `cargo xtask test`：1103 项运行，1100 通过、3 失败、6 跳过。没有修改失败预期、过滤配置或相关实现来制造全绿。
- 用户明确授权后，仅在同一最新 trunk 基线上复测这 3 项，均以相同原因失败：
  - `passthrough::all_passthrough_shaders_binary`：找不到 `dxc`。
  - `passthrough::all_passthrough_shaders_source`：找不到 `dxc`。
  - `subgroup_operations::subgroup_operations`：测试配置期望特定错误，但本机执行实际通过。

新增 GPU 回归覆盖：3 mip × 2 layer、已初始化窄 view 与未初始化的其余范围、同一组的另一张 native texture、重复绑定、未提交即丢弃、编码后 queue 写入、含 attachment 的混合组在 discard 后再读取，以及 texture 销毁后提交报错。已有 zero_init 用例继续覆盖 D3、depth/stencil、部分 copy 等路径。

开发期间修正了两处验证方式：销毁检查最初错误地在编码阶段断言，改为在实际提交验证阶段断言后通过；no_std 初次检查继承仓库默认的调试锁校验配置，但其 ranked locks 依赖 std，显式清空该配置后通过。没有为此修改仓库锁实现。

本轮没有运行 CTS。当前仓库要求明确请求 CTS 才运行，因此不把旧分支 CTS 结果当作本分支的验证证据。实际 GPU 运行限于本机 Metal；Wasm 仅完成编译检查，Vulkan/DX12/GLES 与其他设备仍由对应平台验证。

### 测试审查后补强

原来的单个 `TEXTURE_INIT_COMPLETION` 用例拆到 `tests/tests/wgpu-gpu/zero_init/texture_binding.rs`，分为 `partial_view`、`dropped_command_buffer`、`discard_same_encoder`、`discard_separate_submissions` 和 `destroyed_texture` 五个独立用例。继续使用公开 `wgpu` API、现有 `map_and_read` 和 `wgpu_test::fail`。纹理、shader 和读取设置集中在一个辅助结构中；输出大小由纹理尺寸推导，断言逐 mip/layer/像素检查并报告场景和坐标，移除了手算的 `42`、`16` 和 `36..40`。

两个 discard 用例都先通过 `write_texture` 向各 mip/layer 写入不同的非零值，并读回确认，然后对 mip 1/layer 1 执行 `Load + Discard`。同 encoder 用例在这个 encoder 内继续通过 bind group 执行 compute 读取；跨提交用例先提交 discard，再编码读取。两者都断言目标子资源归零、其余内容保留，同时保留同组另一张非 attachment texture。

这避免了原先 `Clear(WHITE) + Discard` 已知可能掩盖缺失初始化的问题。首次分配的初始化测试仍需要 Linux Vulkan CI 的 `LVP_POISON_MEMORY=true` 加强验证；本机 Metal 结果不能替代该验证。

五个新用例在本机 Metal 上全部实际执行并通过。另做了一次临时故障注入：把不可缓存组的处理改成只检查设备初始化状态，跳过本地 discard 修复。同 encoder 用例在 mip 1/layer 1 的第一个像素读到旧值 `10` 而非 `0`，按预期失败；跨提交用例仍通过。这证明新增用例能够发现原先跨提交测试覆盖不到的错误。故障代码已恢复，四个生产文件的 SHA-256 均与本轮修改前一致。

恢复原实现后，`cargo clippy --all-targets --all-features`、`cargo fmt --all --check` 通过。全量 `cargo xtask test --no-fail-fast` 运行 1107 项：1104 通过、3 失败、6 跳过，五个拆分后的用例全部实际执行并通过。三个失败的诊断与此前已获授权执行的 trunk 对照一致：两个 passthrough 测试缺少 `dxc`，一个 subgroup 测试预期失败但实际通过。该轮测试补强没有再次运行 trunk 测试，也没有改动生产实现或重跑性能计时；后续性能复测见下文。

日志和故障注入前后源码保留在代码工作区的 `target/issue-8518-ready/test-review/`。其上级目录中的 `candidate.patch` 和 `tests.patch` 已更新，完整补丁通过 `git apply --check --reverse` 验证。运行 GPU 测试时，将现有的 `/Users/lijinlei/Rust/forks/wgpu/target/issue-8518/tools` 加入 PATH，以使用该目录中的 `cargo-nextest`。

### Owner 视角审查后的五项收尾

窄 view 测试改为三轮，依次初始化同组另一张纹理、设置完成标记、实际执行缓存命中分支；未提交命令用例删除重复的第二次丢弃。生产代码移除了资格判断中冗余的 `TRANSIENT_ATTACHMENT` 条件，保留 native 范围限制和原有 discard 处理。

专用基准使用 `Scenario` 枚举控制场景，显示名称不再参与行为判断。每组 entries 使用固定数组，全部 entries、descriptor 和结果容器容量均在计时前准备；创建循环复用结果容器，销毁和 GPU 等待留在计时外。两侧对照必须使用修正后的同一份基准。

本轮六项定向检查（五个 GPU 用例和基准测试模式）、clippy、fmt 均通过；全量测试仍为 1107 项运行，1104 通过、3 失败、6 跳过。三个失败与之前的 trunk 对照诊断一致：两项缺少 `dxc`，一项 subgroup 预期失败但实际通过。候选 release 基准也已构建。证据保留在 `target/issue-8518-ready/owner-cleanup/`。

用户随后明确授权复测，已用修正后的同一份专用基准完成 5 对 A/B，每个进程 64 轮。创建 1,000 个 bind group 的耗时中位数由 0.599 ms 增至 0.625 ms（+4.4%）；完整和部分 view 的编码耗时分别降低 7.3% 和 4.5%。旧创建数据混入了临时 entries 分配，原先的 +1.3% 已废弃。最新专用基准数据及其限制见下文；compute/render 表保留首轮记录。

### 本机 Vulkan 与 poison-memory 验证边界

2026-09-19 使用已有的 all-features `wgpu-info` 测试二进制，以 `WGPU_BACKEND=vulkan` 单独枚举适配器，结果为 `devices: []`。另直接加载 `libvulkan.dylib`，得到 `no such file`。当前环境没有可供该程序使用的 Vulkan 运行库及适配器，因此本轮没有执行 Vulkan GPU 回归。报告生成用例正常退出只表示完成了枚举，不能算作 Vulkan GPU 测试通过。探测后已恢复原有 Metal `.gpuconfig`。

这台 Apple Silicon Mac 可以配置 Vulkan SDK/MoltenVK，并启用 wgpu 的 `vulkan-portability` feature，测试 wgpu 的 Vulkan 后端。MoltenVK 将 Vulkan 调用转换到 Metal，能够补充后端路径覆盖，但不能替代 Linux Lavapipe 的内存污染验证。[MoltenVK 官方说明](https://github.com/KhronosGroup/MoltenVK#introduction-to-moltenvk)

`LVP_POISON_MEMORY=true` 是 Mesa **Lavapipe 驱动**读取的选项，不是所有 Vulkan 驱动都支持的开关。其普通分配路径会向新内存填入非零字节，使遗漏初始化的测试更容易失败；单独给 Metal 或 MoltenVK 设置该变量不会获得这一能力。[Mesa 实现](https://chromium.googlesource.com/external/gitlab.freedesktop.org/mesa/mesa/+/f69fbb7fd7b29872e96f6fce066cae5bf5dc7ab8/src/gallium/frontends/lavapipe/lvp_device.c)

本分支基线的 `docs/testing.md` 介绍了 Linux Vulkan CI 使用该选项；`.github/workflows/ci.yml` 全局设置它，`.github/actions/install-mesa/action.yml` 在 Linux 选择 `libvulkan_lvp.so`，同时在 Windows 安装 `vulkan_lvp.dll` 并通过 `VK_DRIVER_FILES` 选择 Lavapipe。因此，这项内存污染验证不限定 Linux，也可在 Windows 原生运行；需确认实际选中了 Lavapipe、实际执行了用例。Linux 平台覆盖与 poison-memory 机制覆盖应分别记录。[wgpu 测试说明](https://github.com/gfx-rs/wgpu/blob/f12c3e4508cb706c1e1faeb973bdb9c6970f5e15/docs/testing.md#memory-initialization-testing)

本轮只检查已有环境，没有安装 Vulkan SDK 或另建 Linux 环境。探测证据位于 `target/issue-8518-ready/owner-cleanup/vulkan-probe.json`、`vulkan-probe.log` 和 `vulkan-loader-probe.log`。

## 性能方法与证据

Apple M1 Max / Metal，macOS 27.0，Rust 1.95.0。基线与候选使用完全相同的 benchmark 源码与构建配置，分别保存可执行文件后串行计时。每个场景 5 对独立进程，交替基线→候选和候选→基线，每次 64 轮；固定 `RAYON_NUM_THREADS=8`。计时关闭额外 backend/Metal 调试验证，保留 core API 验证。本任务的构建、测试与 profiler 不与保留的计时结果并行。

Compute/render 与专用 texture 基准都在计时前完成两轮初始化、提交及等待。编码与 submit 分别测量，GPU 等待不计入，结果不代表 GPU 执行时间或应用帧率。无提交场景不做初始化预热，专门测未提交命令的原始路径。

专用基准每轮绑定 1,000 个组十次，总计 10,000 次 dispatch；创建场景每轮创建 1,000 个组。覆盖完整 view、只有 mip 1/layer 1 的部分 view、全 attachment、attachment/native 混合、无 texture 对照及不提交的首次初始化路径。

最初专用基准一次创建 20,000 张、每张 3 mip × 2 layer 的纹理，trunk 在预热前触发 Metal `Failed to allocate IOGPUDeviceShmem`。该运行无有效计时，已隔离到 `ab-invalid-resource-limit/`；随后改为上述固定 dispatch 数、较少资源的负载，并为两边重新构建。另一次基线运行因采集脚本误用当前目录而未形成配对，已完整保留在 `ab-incomplete-output-path/`；修正为从各二进制编译时固定的目录读取结果后，重新开始整批五对测量，最终表格只使用 `ab/`。

下表为各进程 64 轮均值的五组中位数；变化为 `(候选 / 基线 - 1) × 100%`，负数表示更快。编码加提交先按同一个进程相加，再取中位数。没有把这些数据解释为统计显著性或真实应用帧率。

### 首轮 compute/render 编码结果

以下表格保留首轮测量，本次没有重跑这两套基准。

| 场景 | 基线编码 ms | 候选编码 ms | 编码变化 | 编码+提交变化 |
| --- | ---: | ---: | ---: | ---: |
| compute Encoding (2 passes) | 28.314 | 27.045 | -4.5% | -3.9% |
| compute Encoding (4 passes) | 28.062 | 26.607 | -5.2% | -4.5% |
| compute Encoding (8 passes) | 28.711 | 27.167 | -5.4% | -4.7% |
| compute Encoding (2 threads) | 14.806 | 14.080 | -4.9% | -3.9% |
| compute Encoding (4 threads) | 8.643 | 8.278 | -4.2% | -3.4% |
| compute Encoding (8 threads) | 6.989 | 6.551 | -6.3% | -4.1% |
| render Encoding (1 passes) | 43.092 | 41.052 | -4.7% | -4.5% |
| render Encoding (2 passes) | 41.476 | 39.104 | -5.7% | -5.3% |
| render Encoding (4 passes) | 41.650 | 39.960 | -4.1% | -3.7% |
| render Encoding (1 threads) | 42.239 | 39.675 | -6.1% | -5.6% |
| render Encoding (2 threads) | 21.844 | 20.333 | -6.9% | -5.9% |
| render Encoding (4 threads) | 12.563 | 11.826 | -5.9% | -4.5% |

上述 12 个场景的 60 次配对编码比较全部改善。Compute 中位数改善 4.2%～6.3%，render 改善 4.1%～6.9%；编码加提交也均改善。8 线程 compute 的五对编码变化为 −6.8%、−5.8%、−5.2%、−3.4%、−7.6%，本次没有出现上一轮那样的反向大幅波动；不能据此把改善原因单独归因于线程池设置。

### 修正版 Texture Initialization A/B 复测

2026-09-19，在上述 `f12c3e4508cb706c1e1faeb973bdb9c6970f5e15` 基线上复测。基线只应用同一份 benchmark 文件，`wgpu-core` 无差异；候选使用五项收尾后的生产代码。两侧构建均为 bench profile：`opt-level=3`、`debuginfo=2`、关闭 debug assertions、无额外 benchmark features。先保存各自二进制，再按基线→候选、候选→基线交替运行，共 10 个独立进程，每个进程 64 轮，全部正常退出。

专用源码 SHA-256 两侧均为 `0e2999083840146d4358d1108dab46f86533196941f52196f5639f02e2f84a80`。结果来自各自编译时工作区的输出目录；原始数据和二进制摘要保留在 `owner-cleanup/ab/`，未与首轮数据合并，也未剔除较慢的配对。

| 专用场景 | 基线 ms | 候选 ms | 变化 | 编码+提交变化 | 五对变化 % |
| --- | ---: | ---: | ---: | ---: | --- |
| Create bind groups（1,000 个） | 0.599 | 0.625 | +4.4% | — | -2.3 / +5.7 / +11.9 / +4.4 / +10.1 |
| Full view: encoding | 6.208 | 5.754 | -7.3% | -7.0% | -3.8 / -5.8 / -6.0 / -8.3 / -4.8 |
| Partial view: encoding | 13.008 | 12.419 | -4.5% | -4.3% | -2.8 / -4.7 / -12.1 / -2.3 / -6.5 |
| Attachment: encoding | 6.325 | 6.219 | -1.7% | -1.6% | +4.7 / -0.8 / -2.4 / -2.5 / -3.3 |
| Mixed: encoding | 6.337 | 6.249 | -1.4% | -1.3% | +1.7 / +1.0 / -3.4 / -3.1 / +1.2 |
| Unsubmitted | 5.961 | 5.912 | -0.8% | — | -2.5 / -0.2 / +3.4 / -4.1 / -10.7 |
| No textures: encoding | 2.775 | 2.886 | +4.0% | +3.8% | +0.2 / +10.3 / +0.8 / +29.2 / -5.4 |

创建成本增加约 0.026 ms / 1,000 组，折合约 26 ns / 组；五对中四对变慢。这是修正版基准的观测成本，不能再写成“创建无收益也无损失”，也不能仅凭这项总耗时把成本全部归因于 Box slice 收缩分配。频繁创建而很少复用 bind group 的负载，需要考虑这一取舍。

完整 view 和部分 view 的五对编码比较均改善；计入提交后仍改善。完整 view 的 submit 中位数从 0.135 ms 增至 0.143 ms（+6.0%），部分 view 从 0.462 ms 增至 0.473 ms（+2.4%），表中的编码+提交列已包含这些成本。最新编码收益小于首轮专用基准的 14.2% / 9.0%，不能将不同批次直接混合，或据此推断某一项收尾修改造成了差异。

无 texture 对照中位数变慢 4.0%，且配对范围为 −5.4%～+29.2%；第 4 对为 2.815 → 3.638 ms。该结果有明显波动，但仍需完整报告，不能以“噪声”为由忽略，也不能证明空列表路径没有回归。Attachment、mixed 和未提交场景同样不是每对都改善，不宣称这些控制路径获得稳定收益。

本次复测没有修改生产代码或测试源码，也没有重跑全量测试、trunk 正确性测试或 CTS；正确性结果仍为上一节针对当前代码的最近一次验证。此次新增证据仅包括修正版专用基准与本机 Vulkan 环境探测。

**提交前结论：本机证据支持重复绑定的热路径收益，同时存在创建成本与对照波动，尚缺 Linux Lavapipe poison-memory 验证。** 新分支与 buffer 原型隔离，核心实现缩小为 bind group 范围缓存，相关原生测试与静态检查通过，三个全量失败已在 trunk 同原因复现。可以如实提交这些证据供维护者评估性能取舍，不能宣称全路径无回归或已完成跨后端验收。其他后端、CTS、真实应用帧率及完整 folding 均不在已验证结论内。

可复核证据位于新工作区的 `target/issue-8518-ready/`：`core.patch`、构建 JSON、固定基准二进制、`ab/manifest.json`、逐进程日志/JSON、`ab/summary.json`、原生测试日志、三项 trunk 对照日志及编译检查日志。临时产物均在忽略的 target 中。

本次复测使用其下的 `owner-cleanup/`：`baseline-build.jsonl`、`candidate-build.jsonl`、`baseline-bench`、`candidate-bench`、`run_ab.py`、`ab.log`，以及 `ab/manifest.json`、`ab/build-profiles.json`、10 份逐进程日志和 JSON、`ab/summary.json`。首轮上级目录的产物保留不动。

复现正式基准：

```sh
WGPU_BACKEND=metal WGPU_VALIDATION=0 MTL_DEBUG_LAYER=0 MTL_SHADER_VALIDATION=0 \
RAYON_NUM_THREADS=8 cargo bench -p wgpu-benchmark --bench wgpu-benchmark -- \
  'Texture Initialization' --iters 64
```

进行 A/B 时应先把相同 benchmark 改动应用到基线，并分别构建、保存二进制；只在两边都完成构建后测量，不能在同一 target 目录覆盖二进制后误用候选作为基线。
