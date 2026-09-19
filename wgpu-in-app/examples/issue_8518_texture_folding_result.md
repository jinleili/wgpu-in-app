# wgpu #8518：command buffer 局部 folding 实验

日期：2026-09-19。前一候选与数据见[永久完成缓存版本](issue_8518_texture_ready_result.md)。本轮针对附件纹理、附件与普通纹理混合组，以及多组共享纹理和重叠 view，建立独立实现和三方性能对照。

**状态：功能原型及本机正确性检查完成；性能验收尚未通过，暂不替换上一候选。** 两批测量对共享附件给出了相反方向，完整数据与限制保留在下文。

后续已补充真正的附件写入、后续 render pass 采样和 discard 流程，收敛测量波动，并减少局部范围合并的分配。新数据见[render 流程与局部 folding 成本复核](issue_8518_texture_render_result.md)。本文保留最初 compute 基准的数据与当时判定，不代表后续版本的最终结果。

## 范围与完成标准

- 基线仍为 `f12c3e4508cb706c1e1faeb973bdb9c6970f5e15`，避免把上游变化混入优化对照。
- 分支：`codex/issue-8518-texture-folding`。
- 工作区：`/Users/lijinlei/Rust/forks/wgpu-issue-8518-folding`。
- 原 `codex/issue-8518-texture-ready` 分支及其未提交改动保留不动。
- 本页记录的是提交前的实验状态；后续代码已提交并推送，当前交付见 [Windows 验证交接](issue_8518_windows_validation_handoff.md)。未创建 PR。

完成标准是：在附件和共享纹理场景中实现局部初始化需求合并；保留 discard 后就地清零及提交顺序语义；完成相关回归、静态检查和全量原生测试；使用相同基准比较 trunk、永久缓存和局部 folding 的编码与提交成本。尚未完成 Linux Lavapipe poison-memory 验证，不以 Metal 的通过代替它。

这是 **command buffer 局部首次使用合并**。它没有把初始化状态合入 `TextureUsageScope` 和 usage/barrier tracker，没有替换 device tracker，也没有实现维护者设想的完整 scope → command buffer → device folding。

## 实现

生产改动仅涉及 `command/memory_init.rs`、`command/pass.rs`、`init_tracker/texture.rs`。不修改 `BindGroup`、`Texture`、`Buffer` 的字段，不保留上一候选的永久完成标志或 Box slice 改动。

### 首次使用要求

`CommandBufferTextureMemoryActions` 按 texture tracker index 保存本 command buffer 已登记的范围。第一次访问记录原来的 `MemoryInitKind`；已覆盖的范围不再重复产生初始化 action。

单个矩形范围直接内联保存。新增范围包含原范围时扩展摘要；出现更复杂的覆盖关系时才分配现有的 `TextureInitTracker`，利用它按 mip/layer 跟踪未登记区域。返回的 action 范围可以保守地包含已登记区域，仍按原顺序在提交阶段处理，已处理的子资源不会再次清零。

这里记录的是“首次使用要求已登记”，不是“设备内存已经初始化”。编码阶段不再为这些 texture actions 获取设备初始化状态读锁；是否需要实际清零，留给提交阶段现有 `initialize_texture_memory` 与 device 状态合并。丢弃 command buffer 不会发布全局完成状态。

### 同组复用与跨组共享

command buffer 内用位集合记录已处理过的 bind group。没有 pending discard 时，重复组可以跳过 texture action 遍历。不同组仍会通过 texture index 和 view 范围合并，因此附件组、混合组和共享 view 都能使用这条路径。

索引的生命周期由现有 command buffer 资源引用保证：绑定的组由原 tracker 保留，首次 action 保留 texture 引用。局部摘要随 command buffer 销毁，不跨 command buffer 复用。

### Discard 与顺序

原有 discard 列表、逐 surface 匹配、`fixup_discarded_surfaces`、提交后的 discard 状态更新，以及 D3 depth slice 修复均保留。只要还有 pending discard，就不能因为组已经登记而直接返回。

`读取 → discard → 再读取` 仍在第二次读取前清零；不会把这次清零移到 command buffer 开头。局部首次使用摘要也不会被误认为 discard 后仍有效的内容状态。首次使用的设备查询推迟到提交阶段，按照实际提交顺序处理，而不是固化编码时看到的状态。

## 验证

- 定向 zero-init 与范围合并检查：54 项运行，54 通过。七个 `texture_binding` GPU 用例均在 Apple M1 Max / Metal 实际执行。
- 扩展后的 `Texture Initialization` 基准测试模式单独通过。
- `cargo clippy --all-targets --all-features` 通过。
- `RUSTFLAGS='' cargo check -p wgpu-core --no-default-features` 通过。
- `RUSTFLAGS='' cargo check -p wgpu-core --target wasm32-unknown-unknown --no-default-features` 通过。
- 全量 `cargo xtask test --no-fail-fast`：1110 项运行，1107 通过、3 失败、6 跳过。

三个失败仍为 `passthrough::all_passthrough_shaders_binary`、`passthrough::all_passthrough_shaders_source` 缺少 `dxc`，以及 `subgroup_operations::subgroup_operations` 预期失败但实际通过。诊断与此前用户授权执行的同提交 trunk 对照一致；本轮没有重跑 trunk 正确性测试，也没有修改失败预期。

本轮新增 `overlapping_views` 与 `reordered_submissions`，分别覆盖不同组的部分视图合并，以及先编码读取、后编码 discard、实际先提交 discard。原 `discard_same_encoder` 增加同 encoder 内 discard 之前的读取，确保此前登记过的组不会绕过中途修复。

CPU 用例 `first_uses_preserve_order_and_gaps` 枚举 3 mip × 3 layer 上的矩形组合，以及首次读取/完整写入的组合，共 5,184 种序列；逐子资源比较合并前后的首次初始化要求，并确认完整覆盖后不再产生需求。该检查不依赖 GPU 新分配内存是否恰好为零，但不能替代完整 GPU poison-memory 验证。

开发时首次编译因新增单元测试缺少 `alloc::vec::Vec` 导入失败；补上导入后，定向检查和全量测试均重新执行。没有修改 GPU 后端实现或测试预期来规避错误。

## 三方性能对照

Apple M1 Max / Metal，沿用上一轮的 bench 构建配置。三者都从同一工作区、同一提交和同一份 benchmark 源码构建，分别固定二进制：trunk 原实现、上一候选的永久完成缓存、当前局部 folding。构建脚本在结束时恢复 folding 源码，并核对生产补丁和二进制摘要。

每个版本运行五个独立进程，每进程 64 轮，交错执行三者的顺序。固定 `RAYON_NUM_THREADS=8`，关闭额外 backend/Metal 调试验证，保留 core API 验证。正式计时期间不运行本任务的构建、测试或 profiler。

每轮仍为 1,000 个 bind group、10,000 次 dispatch。保留完整 view、部分 view、附件、混合、未提交和无 texture 对照；新增场景让 1,000 个组共享 16 对 texture，分别使用完整附件 view，以及重叠两级 mip 的附件/普通纹理混合 view。创建场景仍创建 1,000 个组，entries、descriptor 和结果容器容量在计时外准备。

编码、queue submit 分别计时，GPU 等待排除在外。表格为五个进程均值的中位数；编码+提交先在同一进程相加，再取中位数。负数表示耗时减少。这些数据不代表真实应用帧率或 GPU 执行时间。

| 场景 | trunk ms | 永久缓存 ms | 局部 folding ms | folding / trunk | folding / 永久缓存 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full view: encoding | 6.342 | 6.063 | 6.223 | -1.9% | +2.6% |
| Create bind groups | 0.576 | 0.608 | 0.572 | -0.7% | -5.9% |
| Partial view: encoding | 13.019 | 12.064 | 12.513 | -3.9% | +3.7% |
| Attachment: encoding | 6.545 | 6.372 | 6.040 | -7.7% | -5.2% |
| Mixed: encoding | 6.095 | 6.317 | 6.143 | +0.8% | -2.7% |
| Unsubmitted | 5.903 | 5.806 | 5.345 | -9.5% | -7.9% |
| No textures: encoding | 3.451 | 2.955 | 2.921 | -15.4% | -1.2% |
| Shared attachments: encoding | 4.061 | 4.044 | 4.597 | +13.2% | +13.7% |
| Overlapping mixed views: encoding | 7.000 | 6.790 | 6.580 | -6.0% | -3.1% |

上表的 encoding 为编码时间，Create 为创建时间，Unsubmitted 为编码后丢弃且不提交。下表将同进程的编码与提交相加：

| 场景 | 合计相对 trunk | 合计相对永久缓存 | 五对编码变化，相对 trunk % |
| --- | ---: | ---: | --- |
| Full view | -0.4% | +3.9% | -3.0 / +1.0 / +0.7 / +1.9 / -5.1 |
| Partial view | -3.3% | +4.6% | -4.8 / -3.9 / -2.3 / -7.4 / -2.4 |
| Attachment | -6.7% | -4.0% | -1.6 / -7.4 / -1.3 / -7.7 / -13.4 |
| Mixed | +1.8% | -1.7% | -3.0 / +1.4 / +3.2 / +6.5 / -4.9 |
| No textures | -15.1% | -1.1% | -31.1 / +11.3 / -18.5 / -5.3 / -8.9 |
| Shared attachments | +13.2% | +13.7% | +2.6 / -3.0 / +15.9 / -6.7 / +16.6 |
| Overlapping mixed views | -5.9% | -3.0% | -4.7 / -1.3 / -7.8 / -8.7 / -8.8 |

附件和重叠 view 的编码五对均改善，编码+提交相对 trunk 分别减少 6.7% 和 5.9%；未提交场景也五对改善。局部合并确实能够覆盖上一候选排除的场景。

但混合组的编码+提交增加 1.8%，共享附件增加 13.2%。共享附件五对编码变化为 +2.6%、−3.0%、+15.9%、−6.7%、+16.6%，方向并不一致。完整和部分 view 相对永久缓存的合计开销增加 3.9% / 4.6%，因此这版不能无条件替代上一候选。

全 view 的 submit 中位数由 0.138 ms 增至 0.219 ms，附件由 0.142 ms 增至 0.198 ms，混合由 0.139 ms 增至 0.206 ms。省去编码阶段查询后，首次需求仍要在提交阶段查询设备状态；必须把这些成本算入净收益。

创建耗时相对 trunk 为 −0.7%，配对仍有正负变化，不据此宣称创建优化。无 texture 对照看似改善 15.4%，但配对范围为 −31.1%～+11.3%，显然不能将这一幅度归因于 texture 初始化优化。这也是本轮性能结论必须保守的原因。

### 针对共享附件的诊断

另用仅加入阶段标记的基准，在保持完整场景顺序的情况下对共享附件阶段做原生 `sample` 采样。采样运行使用 256 轮，数据单独保存，未并入上面的正式计时。候选快照中没有看到足以解释 13% 回退的明确新增初始化热点；基线采样结束时已经进入下一个场景，不能做严格的阶段对照或据此推导精确耗时占比。

这些采样不足以确定回退来自某段新增逻辑，也不足以证明它只是噪声。阶段标记已经移除，benchmark 源码按字节恢复；没有依据这次采样盲目调整生产实现。

针对上述回退和波动，使用首轮原始固定二进制追加三对无采样复核，保持完整场景顺序、每进程 64 轮，交替 baseline/folding 顺序。没有重建生产代码或更换输入，结果独立保留，没有替换首轮数据。

| 场景 | 复核编码/创建变化，相对 trunk | 复核编码+提交变化 | 三对编码/创建变化 % |
| --- | ---: | ---: | --- |
| Full view: encoding | -6.3% | -5.2% | -6.6 / -9.1 / +0.3 |
| Create bind groups | +2.3% | — | -6.2 / +2.3 / +5.6 |
| Partial view: encoding | -3.6% | -2.6% | -4.8 / -3.8 / -3.1 |
| Attachment: encoding | -3.4% | -2.0% | -6.3 / +1.2 / -4.6 |
| Mixed: encoding | -6.9% | -5.8% | -9.1 / -4.9 / -6.7 |
| Unsubmitted | -8.9% | — | -6.8 / -8.9 / -11.0 |
| No textures: encoding | -8.6% | -8.4% | -8.6 / +2.7 / -12.3 |
| Shared attachments: encoding | -6.6% | -6.5% | -4.8 / -5.4 / -6.6 |
| Overlapping mixed views: encoding | -7.0% | -6.9% | -9.0 / -0.6 / -7.1 |

共享附件三对编码均改善，中位数为 −6.6%，没有重复出首轮的 +13.2% 回退；但无 texture 对照仍为 −8.6%，配对方向也不一致。因此，既不能把首轮 +13.2% 当作已确认的固定回退，也不能只采用复核的改善数字宣布通过。创建场景本轮为 +2.3%，与首轮 −0.7% 同样存在反向波动。

部分 view、重叠 view 和未提交场景在两批编码配对中均改善，说明这一方向有继续验证的价值。共享、混合及无 texture 对照的跨批次波动，加上提交阶段增加的成本，使本机数据还不足以给出稳定的整体收益承诺。

**当前判定：局部 folding 的功能原型与本机正确性验证已完成，性能验收尚未通过。** 保留独立分支和全部对照证据，不替换永久缓存版本，也不据此关闭 #8518。需要继续验证的是更完整的 scope 合并能否减少额外局部状态维护成本，以及目标应用中的总 CPU 收益，而不是把编码下降直接等同于整体优化。

## 尚存边界

局部摘要需要额外的 command buffer 内存：稠密数组的大小受本次访问的最高 texture index 影响，bind group 位集合也受索引范围影响；复杂 view 覆盖还会分配范围 tracker。它避免了永久缓存对附件纹理的排除，但不能假定局部状态维护成本为零。

本轮是专用 compute 基准对照，没有重新测量整套 compute/render 编码基准，也没有真实应用验收。Linux Vulkan/Lavapipe、其他后端与 CTS 均未运行。当前 Mac 没有可用 Vulkan 运行库的探测结果见[上一轮记录](issue_8518_texture_ready_result.md#本机-vulkan-与-poison-memory-验证边界)。

## 复核产物

代码工作区的 `target/issue-8518-folding/` 保存生产补丁、三份固定基准二进制、构建 JSON 和日志、`build-manifest.json`、`build_variants.py`、`measure.py`、`abc/manifest.json`、逐进程 JSON/日志、`abc/summary.json`，`confirm/` 中的追加三对复核、`profile-shared/` 中的独立采样，以及定向、全量、clippy、no_std 和 Wasm 验证日志。临时产物不进入文档目录或提交内容。
