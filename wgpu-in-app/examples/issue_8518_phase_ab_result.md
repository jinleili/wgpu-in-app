# wgpu #8518：阶段 A、B 实施与验证记录

日期：2026-09-18。配套方案：[详细实施方案](issue_8518_implementation_plan.md)。

2026-09-19 后续：texture 已在独立分支开展实验，见 [Texture 后续实验](issue_8518_texture_result.md)。本记录中的 A/B 数据与 buffer 候选结论保持；B 没有普遍收益不构成否定 texture 优化的证据。

## 1. 交付范围与版本

本次完成当前版本审计、基线采样、buffer 初始化完成摘要、统一调用迁移、回归测试及 A/B 基准。**B 是已实现的实验候选，性能验收未通过，暂不建议合入。**Texture 的快速路径与 discard 状态机属于 C/D，本次没有实施。

| 项目 | 值 |
| --- | --- |
| wgpu 基准提交 | `f7c40b2cf96327b9d2dd2ffef73400528d2ec4f7` |
| 实现分支 | `codex/issue-8518-buffer-init` |
| 实现提交 | `3ba57ab26244044dcfe789add07b33df7b5d64b7`（实验候选，性能验收未通过） |
| 实现目录 | `/Users/lijinlei/Rust/forks/wgpu-issue-8518` |
| 对照目录 | `/Users/lijinlei/Rust/forks/wgpu-8518-baseline`，同一提交 detached HEAD，只增加相同基准代码 |
| 原 checkout | `/Users/lijinlei/Rust/forks/wgpu`，源文件保持原状 |
| 测试硬件 | Apple M1 Max / Metal |
| 系统、工具链 | macOS 27.0 (26A428)，rustc 1.95.0 (59807616e 2026-04-14) |
| CTS revision | `83cbc838309ba7bddf6f3ec5c17574564773732b`，匹配 `cts_runner/revision.txt` |
| 交付状态 | 代码已 commit，配套文档随本次提交存档；均未 push，未创建 PR；应用工程的依赖路径未切换 |

应用仓库原有 Apple 工程、依赖文件和其他 issue 示例的未提交修改保留。本次实现位于独立 wgpu worktree；在应用工程中运行原有命令，不等于已经使用本优化。

## 2. 阶段 A：当前证据与投入判断

### 2.1 CPU 采样

先对原始 wgpu 运行现有 `Computepass Encoding`，用 macOS `sample` 以 1ms 间隔采样 8 秒。运行包含 10,000 次 dispatch，以及单线程多 pass、多线程编码。

本机采样可以观察到初始化检查，但主要热点包括 Metal resource binding 和 texture 跟踪。没有复现 issue 历史报告的“初始化 RwLock 占 10–20%”这一比例。`flush_bindings_helper` 的 inclusive 时间包含 HAL 绑定、buffer 检查和 texture 检查，不能全部算成 buffer 读锁成本。

初始采样使用原有基准，首轮初始化和旧 `previous` 文件会干扰日志里的百分比；该次数据只用于定位热点，不作为最终优化幅度。正式对照使用两边完全相同的基准，并在计时前完成初始化和 GPU 等待。

阶段 A 的结论是：存在可消除的 buffer 热路径读锁，适合交付规模受控的 B 验证收益；当前采样不足以支持扩大到 texture，更不能预先承诺 10–20% 的端到端提升。

### 2.2 查询入口审计

所有 buffer 初始化查询经过 `BufferInitTracker::create_action` / `check_action`，最终由私有 `check` 决定是否拿读锁。迁移 24 处调用，原 range、action kind、资源验证及 barrier 保持。

| 文件（相对实现目录） | 查询数 | 入口 |
| --- | ---: | --- |
| `wgpu-core/src/device/resource.rs` | 1 | 创建 bind group |
| `wgpu-core/src/command/pass.rs` | 1 | compute/render 共享绑定处理 |
| `wgpu-core/src/command/compute.rs` | 1 | indirect dispatch |
| `wgpu-core/src/command/render.rs` | 6 | vertex/index/indirect/count/bundle replay |
| `wgpu-core/src/command/bundle.rs` | 3 | bundle 录制 |
| `wgpu-core/src/command/transfer.rs` | 6 | buffer copy、texture copy 的 buffer 侧及对齐范围 |
| `wgpu-core/src/command/clear.rs` | 1 | clear |
| `wgpu-core/src/command/query.rs` | 1 | resolve query |
| `wgpu-core/src/command/ray_tracing.rs` | 4 | vertex/index/transform/AABB 输入 |

### 2.3 状态更新与发布审计

| 位置 | 现有语义和本次处理 |
| --- | --- |
| 普通 buffer 创建 | 仍以 `tail_start` 建立待初始化范围；不改变 padding 清理 |
| invalid / HAL 导入 buffer | 仍使用长度为 0 的初始化 tracker；初始摘要为完成 |
| `device/resource.rs` mapped-at-creation staging 路径 | staging 已补零后移除对应范围；map state 继续禁止未 unmap 的 buffer 被正常提交使用 |
| `device/mod.rs::map_buffer` 普通分支 | 补零、必要 flush、完整消费 drain 后发布；保持 CPU/GPU 映射语义 |
| 非一致性且没有 MAP_WRITE 的只读 map | 只遍历 `uninitialized`，不 drain；CPU 临时补零不会使摘要错误地变成完成 |
| `device/queue.rs::PendingWrites::clear_buffer` | 原有 HAL clear 编码、pending resource 登记后再更新 tracker |
| `device/queue.rs::write_staging_buffer_impl` | 原有 copy 编码、pending resource 登记后再更新 tracker；队列 pending-writes 锁保持 |
| `command/memory_init.rs::initialize_buffer_memory` | 按 submission 顺序处理 action；每次 tracker 更新闭包结束后判断完成，清理范围合并、清零及 barrier 保持 |

提交序列仍由 `Queue::allocate_submission` 取得的 `command_indices.write()` 等同步机制保护。全局状态更新开始后若提交准备失败，既有 `lose_device_on_error` 路径继续令设备失效。此处的“完成”表示初始化已经在原有时序体系下得到保证，并不表示 GPU 已执行完毕。

## 3. 阶段 B：实现与正确性边界

核心文件：[buffer.rs](/Users/lijinlei/Rust/forks/wgpu-issue-8518/wgpu-core/src/init_tracker/buffer.rs)。

```text
BufferInitTracker
  ├─ fully_initialized: AtomicBool
  └─ tracker: RwLock<InitTracker<BufferAddress>>

check(range):
  flag == true  → None，无需读锁
  flag == false → 原 tracker.read().check(range)

with_tracker_mut(update):
  write lock → update → drain/迭代器结束
             → tracker 没有有效待初始化范围时，发布 true
```

1. 正式代码只有 `false → true`，不清除该标志。Buffer 初始化不可回退，区别于有 discard 的 texture。
2. 原子读写采用 `Relaxed`。读者命中 true 后直接返回，不读取任何未加锁的 tracker 内容；该标志不承担普通内存数据发布、queue 顺序或 GPU 完成同步。
3. 读到保守的旧 false，只会多走一次既有锁内检查；不会漏初始化。所有 range 数据仍在同一个锁内。
4. `with_tracker_mut` 不返回锁 guard。闭包的返回类型不能携带从参数借出的 drain，完成摘要的检查发生在 drain 的生命周期结束之后。
5. `InitTrackerDrain` 是惰性更新：完整消费和未开始消费即 drop 的行为不同于部分消费即 drop。实现依据实际范围容器发布，保留原有 drain 算法。
6. `InitTracker::new(0)` 可能保留 `[0..0]`，因此完成判断使用“所有范围为空”，不能只检查容器长度。
7. 已完成后不反复写入原子标志；初始化更新的锁和验证仍保留。本次没有尝试让写路径无锁。
8. destroyed/mapped/device-lost 检查位于原有资源/提交验证流程，快速路径不会跳过它们。

### 内存成本

使用独立小程序，以同一 `std + metal + wgsl` feature 配置测 `size_of` / `align_of`：

| 类型 | 基线大小 | B 大小 | 对齐 |
| --- | ---: | ---: | ---: |
| `wgpu_core::resource::Buffer` | 272 B | 280 B | 8 B，保持 |
| `wgpu_core::resource::Texture` | 680 B | 680 B | 8 B，保持 |

一个 AtomicBool 在本配置下引入 8 字节的结构体增量，约 +2.94%；10 万个 buffer 约增加 0.76 MiB，仅指 core 对象大小。其他平台、features 和底层分配器占用需另测。

## 4. 正确性与构建验证

### 4.1 新增回归

内部新增六项测试，与原 init-tracker 测试合计 17 项：

- 局部初始化与逐段完成。
- drain 完整/部分消费的发布边界。
- 仅检查待初始化范围不发布完成。
- 空 tracker 和非零 tracker 的 stale-false 回退。
- barrier 控制的并发查询与完成发布。
- 完成后持有 tracker 写锁时，另一线程查询仍可返回，证明查询确实绕过读锁；超时失败前先释放锁，避免永久死锁。

GPU 新增 `buffer_init_submission_order`：五种数据路径，每条执行三轮 compute 加一并 readback 比对全部内容。

| 路径 | 验证内容 |
| --- | --- |
| partial | 中间写入 7，前后为零，后续多轮累加不被清零 |
| fragmented | 四块以非顺序写满，最终内容完整 |
| write_after_encoding | 先编码读取，再 queue 写入，最后提交；既有 action 不破坏新写入 |
| dropped_encoder | clear 编码后丢弃 command buffer，再读取仍正确初始化 |
| mapped | mapped-at-creation 局部写入，unmap 后全范围内容正确 |

同时运行现有 dynamic-offset、vertex/index、tail padding、零长度 binding、texture-to-buffer 等初始化用例及全仓库回归。Metal 的零分配可能掩盖漏清零，GPU 回读结论由内部范围/发布测试补充；尚未用 Vulkan/llvmpipe poison memory 验证。

### 4.2 实际结果

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all --check`、`git diff --check` | 通过 |
| `cargo clippy --tests`；另对 `wgpu-benchmark` bench target 执行 Clippy | 均通过，最终日志无 warning/error |
| `cargo test -p wgpu-core --lib init_tracker` | 17/17 通过 |
| `RUSTFLAGS="--cfg wgpu_validate_locks"` 的 init_tracker 测试 | 17/17 通过，使用独立 target |
| `cargo check -p wgpu-core --no-default-features` | 通过 |
| 新增 `Buffer Initialization` benchmark test mode | 通过，含后补的 write 场景 |
| `cargo xtask test`，Metal | 1104 项运行，harness 报 1101 pass、3 fail、6 skip；失败见下一节 |
| `zero_init` 模块在整体测试中的结果 | 39 项实际执行通过；另 5 项因硬件 features 不支持未执行 |
| buffer 初始化 CTS | 23/23 通过，无 skip/fail |
| 默认 CTS 清单 | 466 个分组有有效 Summary：48,596 pass、3,676 skip、0 fail；另 8 个分组无总结，4 个按原 Metal 规则跳过；不能标记为全量通过 |

harness 的 1101 pass 包含硬件不支持的注册用例和预期失败用例，不能解读成 1101 项 GPU 实际执行成功。`zero_init` 的五项不支持用例主要是 NV12/P010 平面操作。


### 4.3 已有失败

整体测试中的三项失败在未修改 core 的对照 checkout 原样复现：

- `passthrough::all_passthrough_shaders_binary`、`passthrough::all_passthrough_shaders_source`：测试要调用 `dxc`，当前环境未安装该命令。
- `subgroup_operations::subgroup_operations`：测试标记预期某种失败，但本机实际通过，harness 因此报错。

没有修改这些测试的预期、skip 列表或工具链，以免扩大本次范围。整体 suite 因此不能标记为全绿。


### 4.4 默认 CTS 的完整性核对

默认 `test.lst` 有 478 个分组。先顺序取得 202 个有总结的分组，再把剩余清单拆成四份继续执行。最终 466 个分组取得 Summary，其 selector 集合没有重复；4 个由原有 `fails-if(...metal...)` 规则跳过。汇总为 **48,596 通过、3,676 skip、0 fail**，数字采用 runner 顶层 case 的计数，不等于子用例总数。

余下八个分组返回 exit 0 却没有 Summary。对每个分组再用固定的基线和候选 `cts_runner` 串行复跑，得到完全相同的退出码、总结缺失及文本输出：

- `api,operation,adapter,requestAdapter`
- `api,operation,device,lost`
- `api,operation,uncapturederror:iff_uncaptured`
- `api,validation,capability_checks,features,clip_distances`
- `api,validation,capability_checks,limits,maxBindGroups:setBindGroup`
- `api,validation,capability_checks,limits,maxBufferSize`
- `api,validation,capability_checks,limits,maxInterStageShaderVariables`
- `api,validation,error_scope`

其中 uncapturederror 和 error_scope 两组输出 `cts_runner caught WebGPU error: not enough memory left`；其他六组为空输出。这里只能确认是基线也存在的执行/报告限制，尚未定位原因，不能当作测试通过。没有为此修改 runner、CTS 内容或预期列表。

因此本次默认 CTS 的准确结论是“全部清单条目均已调度或按原规则跳过，466 组有完整结果，8 组未取得有效结论”，不是“全套 CTS 全绿”。完整 selector 和复跑数据在 `cts-missing.lst`、`cts-{baseline,candidate}-missing.json`，计数在 `cts-summary.json`。

## 5. A/B 性能结果

### 5.1 测量协议

- release/bench profile，原仓库的 thin LTO + debug symbols；基线和候选固定为不同可执行文件，避免共用 Cargo target 被覆盖。
- 两边使用相同新增 `Buffer Initialization` 以及相同 compute/render warm-up 改动。基准代码不含候选专用分支。Buffer 最终结果使用补充逐并行场景预热后的可执行文件；compute/render 保留首批同源对照结果及其 `*-bench-prewarm` 可执行文件，各场景内部没有混用版本。
- 计时环境设置 `WGPU_BACKEND=metal`、`WGPU_VALIDATION=0`、`MTL_DEBUG_LAYER=0`、`MTL_SHADER_VALIDATION=0`，关闭额外 backend/Metal 调试验证；wgpu-core 的常规 API 验证仍然执行。计时过程中没有 profiler、编译或本任务的 CTS/GPU 测试并行运行。
- 5 组独立进程交替 A/B，奇数组 A→B，偶数组 B→A。Buffer 每组 64 次、现有 compute/render 每组 32 次。每个 JSON 的值是该进程内固定次数的平均耗时，再对五组取中位数与 min–max。
- encoding 和 submit 分开记录；合计取每组两者之和后再求中位数。新增 buffer 基准的 wait 单独记录；既有 compute/render 的 poll 在编码/提交计时之外。等待时间不能作为 CPU 编码收益。
- `initialized`：2,048 bind groups，每组四个 256B buffer，先创建 bind group 再写满，以保留初始化 action。
- `partial`：512B buffer 只初始化/绑定前 256B，尾部持续未初始化，摘要始终 false。
- `shared`：所有 bind groups 共用四个已初始化 buffer；1/2/4/8 个并行编码任务。
- `cold`：每次新建 8,192 个 buffer 与 2,048 bind groups，靠首次使用触发初始化；creation、encoding、submit、wait 分开。
- `write`：同样每次新建资源，再 queue.write_buffer 写满；额外独立记录 write 时间，观察写侧成本。
- 原 compute/render 使用 10,000 dispatch/draw 的既有混合资源路径，作为端到端主场景及回归对照。

### 5.2 Buffer 单独场景

下表变化为 `(B / 基线 - 1) × 100%`，负数表示更快；方括号是五个独立进程平均值的 min–max，不是置信区间。

| 场景 | 基线 encoding，ms［范围］ | B encoding，ms［范围］ | 变化 | encoding+submit，ms | 合计变化 |
| --- | ---: | ---: | ---: | ---: | ---: |
| initialized (1 线程) | 1.702 [1.603–1.752] | 1.681 [1.572–1.886] | -1.2% | 1.832 → 1.836 | +0.2% |
| initialized (2 线程) | 1.124 [1.000–1.209] | 1.264 [1.116–1.340] | +12.4% | 1.348 → 1.519 | +12.7% |
| initialized (4 线程) | 0.805 [0.759–0.852] | 0.843 [0.764–0.896] | +4.7% | 1.131 → 1.190 | +5.2% |
| initialized (8 线程) | 0.631 [0.620–0.909] | 0.634 [0.606–0.759] | +0.5% | 1.023 → 1.045 | +2.2% |
| partial (1 线程) | 1.769 [1.746–1.952] | 1.793 [1.759–1.912] | +1.4% | 1.929 → 1.950 | +1.1% |
| partial (2 线程) | 1.148 [1.114–1.236] | 1.173 [1.059–1.282] | +2.2% | 1.403 → 1.464 | +4.3% |
| partial (4 线程) | 0.833 [0.803–0.857] | 0.830 [0.797–0.862] | -0.3% | 1.183 → 1.235 | +4.4% |
| partial (8 线程) | 0.661 [0.647–0.734] | 0.663 [0.657–1.020] | +0.2% | 1.082 → 1.132 | +4.6% |
| shared (1 线程) | 0.942 [0.921–0.983] | 0.971 [0.926–1.206] | +3.1% | 0.988 → 1.017 | +3.0% |
| shared (2 线程) | 1.068 [0.996–1.172] | 0.873 [0.838–1.120] | -18.2% | 1.137 → 0.939 | -17.4% |
| shared (4 线程) | 1.157 [1.118–1.185] | 0.910 [0.888–0.971] | -21.4% | 1.260 → 1.011 | -19.7% |
| shared (8 线程) | 1.058 [0.987–1.151] | 0.942 [0.741–1.008] | -11.0% | 1.222 → 1.122 | -8.2% |
| cold | 1.816 [1.776–1.831] | 1.805 [1.752–1.861] | -0.6% | 7.262 → 7.389 | +1.8% |
| write | 1.883 [1.877–1.929] | 1.866 [1.807–1.898] | -0.9% | 2.221 → 2.198 | -1.0% |

### 5.3 既有混合 Compute 场景

| 场景 | 基线 encoding，ms［范围］ | B encoding，ms［范围］ | 变化 | encoding+submit，ms | 合计变化 |
| --- | ---: | ---: | ---: | ---: | ---: |
| (2 passes) | 27.996 [27.550–31.436] | 27.946 [27.855–28.302] | -0.2% | 31.079 → 31.074 | -0.0% |
| (4 passes) | 28.936 [27.511–30.706] | 29.171 [28.004–29.644] | +0.8% | 32.457 → 32.625 | +0.5% |
| (8 passes) | 28.475 [28.439–30.921] | 29.019 [28.622–29.450] | +1.9% | 32.771 → 33.285 | +1.6% |
| (2 线程) | 14.702 [14.526–15.944] | 14.803 [14.662–14.909] | +0.7% | 17.784 → 17.873 | +0.5% |
| (4 线程) | 8.682 [8.601–9.159] | 8.729 [8.685–8.791] | +0.5% | 12.022 → 12.023 | +0.0% |
| (8 线程) | 6.911 [6.717–7.644] | 6.853 [6.670–6.886] | -0.8% | 11.077 → 10.970 | -1.0% |

### 5.4 既有 Render 场景

| 场景 | 基线 encoding，ms［范围］ | B encoding，ms［范围］ | 变化 | encoding+submit，ms | 合计变化 |
| --- | ---: | ---: | ---: | ---: | ---: |
| (1 passes) | 44.240 [43.325–44.771] | 42.697 [42.491–44.456] | -3.5% | 47.105 → 45.495 | -3.4% |
| (2 passes) | 41.358 [41.229–42.350] | 41.037 [40.112–41.972] | -0.8% | 45.131 → 44.898 | -0.5% |
| (4 passes) | 41.970 [41.375–44.021] | 41.190 [40.149–41.668] | -1.9% | 46.335 → 45.532 | -1.7% |
| (1 线程) | 42.059 [41.452–42.498] | 41.492 [40.482–42.693] | -1.3% | 44.997 → 44.376 | -1.4% |
| (2 线程) | 21.788 [21.565–21.858] | 21.552 [21.367–21.923] | -1.1% | 25.432 → 25.221 | -0.8% |
| (4 线程) | 12.474 [12.437–12.880] | 12.460 [12.287–12.777] | -0.1% | 16.703 → 16.569 | -0.8% |

### 5.5 创建、写入与等待成本

| 测量项 | 基线中位数，ms | B 中位数，ms | 变化 |
| --- | ---: | ---: | ---: |
| cold: create | 17.201 | 17.078 | -0.7% |
| cold: submit | 5.445 | 5.585 | +2.6% |
| cold: wait | 11.423 | 11.490 | +0.6% |
| write: create | 17.040 | 16.903 | -0.8% |
| write: write | 27.131 | 26.886 | -0.9% |
| write: submit | 0.336 | 0.329 | -1.9% |
| write: wait | 20.272 | 20.217 | -0.3% |

冷路径和队列写入没有出现明确的大幅退化；wait 包含 GPU 和系统调度影响，这些小差异不用于证明编码优化。

### 5.6 复测经过与结论

第一批 buffer 对照虽已做资源 warm-up，但首个多线程子场景仍可能计入 Rayon 启动和并行 encoder 的首次开销。因为独立 buffer 的 2 线程样本出现 +5.8%，随后在两边加入每种线程数自己的编码、submit、poll 预热，再独立重跑五组。最终表使用这批复测数据；首次数据保存在 `ab-prewarm/`，没有混入最终中位数。

预热后同一场景仍为 **+12.4%**，五组配对变化为 +26.6%、+14.8%、−0.7%、+17.7%、+4.5%。不能归因于启动成本，也不能因部分范围重叠就忽略。数据布局、分配大小和生成代码的影响尚未定位；目前只确认 Buffer 类型增加 8B，不能直接声称这就是退化原因。

共享 buffer 的 4 线程结果最有说服力：encoding 中位数 **−21.4%**，五组配对均改善约 16–25%。2/8 线程共享场景也有中位数改善，但波动更大。此负载反复查询同一组早期创建的 bind group，有较强共享读锁竞争；不能推导所有 buffer 使用都会得到相同收益。

若 bind group 创建时 buffer 已经初始化，原版通常就不保留该 buffer 的初始化 action，因此该路径本来就很少执行后续检查。这也是评估真实应用收益时必须核对的资源创建顺序。

既有混合 compute 的 encoding 变化为约 −0.8%～+1.9%，encoding+submit 约 −1.0%～+1.6%；没有达到方案建议的主场景至少 5% 收益。Render 中位数改善约 0.1–3.5%，多数小于各组波动，不能作为明显普遍加速的依据。

**性能验收不通过：当前 B 代码作为实验候选保留，不建议直接合入。** Texture 的投入依据需独立测量，不能从 B 的结果直接外推。没有降低原有性能验收门槛。


## 6. 复现与证据位置

在实现 worktree 下执行：

```sh
cargo fmt --all --check
cargo clippy --tests
cargo test -p wgpu-core --lib init_tracker
cargo check -p wgpu-core --no-default-features
WGPU_BACKEND=metal cargo xtask test
cargo xtask cts --backend metal 'webgpu:api,operation,resource_init,buffer:*'
cargo xtask cts --backend metal
cargo bench -p wgpu-benchmark --no-run
```

本机常规编译将 `CARGO_TARGET_DIR` 设为 `/Users/lijinlei/Rust/forks/wgpu/target`，并把临时安装的 `target/issue-8518/tools/cargo-nextest` 所在目录加入 PATH。跨 checkout 共用 target 时发现 `xtask` 可复用带旧 `CARGO_MANIFEST_DIR` 的 runner，因此最终 CTS 分片使用在候选目录、独立 `target/issue-8518/xtask-runtime` 中编译的 runner，并传入绝对清单路径；前面的原生全量及 buffer CTS 日志已核对为候选路径。正常复现时优先让每个 checkout 使用自己的 target，避免这一问题。CTS 使用现有同 revision 的 checkout，通过 `--skip-checkout` 复用已经生成的 `out`；该选项不能用于版本不匹配的 CTS。

| 证据 | 位置 |
| --- | --- |
| A 原始采样与原版 compute 日志 | `/Users/lijinlei/Rust/forks/wgpu/target/issue-8518/phase-a-sample.txt`、`phase-a-compute.log` |
| 本次所有构建、验证日志 | `/Users/lijinlei/Rust/forks/wgpu-issue-8518/target/issue-8518/` |
| A/B 原始每轮 JSON/log | 上述目录的 `ab/`；首次 buffer 样本另存于 `ab-prewarm/` |
| 两版基准源码快照 | 上述目录的 `bench-source-current/`、`bench-source-prewarm/`；旧版源码的 SHA-256 与最初记录一致 |
| A/B 驱动、汇总脚本 | 上述目录的 `run_ab.py`、`summarize_ab.py`；完整汇总为 `ab-summary.csv` / `ab-summary.json` |
| 固定可执行文件 | 上述目录的 `baseline-bench`、`candidate-bench`；首批 compute/render 对照使用 `baseline-bench-prewarm`、`candidate-bench-prewarm` |
| 类型布局测量程序与日志 | 上述目录的 `layout/`、`layout-baseline.log`、`layout-candidate.log` |
| 未修改 core 的失败复测 | 上述目录的 `baseline-existing-failures.log` |
| CTS 清单覆盖及不完整分组复测 | 上述目录的 `cts-shards.json`、`cts-summary.json`、`cts-missing.lst`、`cts-{baseline,candidate}-missing.json` |

A/B 复跑可执行 `python3 target/issue-8518/run_ab.py`，只跑 buffer 时增加环境变量 `ISSUE8518_SCENARIO=buffer`；须先确认没有本任务的构建、CTS 或其他 GPU 测试在同时运行。随后用 `python3 target/issue-8518/summarize_ab.py` 重新汇总。

详细原始产物都位于忽略的 `target` 中，不应作为普通源码提交。生产改动、内部测试、GPU 测试与可复用 benchmark 已包含在上述实现提交中，便于单独审查。

## 7. 验收结论与后续边界

- A 已完成：取得当前热点证据、完成全部 buffer 初始化读写入口和发布时序审计，没有复现历史 10–20% 占比。
- B 的代码、测试和可重复实验已完成；相关正确性检查通过。整体原生 suite 有三项基线已复现的环境/预期问题；默认 CTS 有八个分组在基线与候选上均缺少有效总结，均已明确列为未验收。
- B 的性能目标未通过：共享竞争场景有效，但通用 compute 没有明显收益，独立 buffer 2 线程出现可复现的退化信号。当前分支应作为待优化/可否决的候选审查，不能标记为已经通过合入验收。
- 若继续改进 B，应先用独立 buffer 2 线程场景验证生成代码与对象布局的影响，并重新做相同 A/B。Texture 已另行开展实验，不把 B 的性能优化作为前置条件。
- GPU 实际执行只覆盖本机 Metal；Vulkan、DX12、GLES、Wasm 的运行正确性、non-coherent 硬件路径和性能尚未验证。`no-default-features` 编译不等于已完成所有 no_std 目标验证。
- 没有改变公开 API 或零初始化语义；未写 changelog。代码与文档已按用户要求提交，保留实验候选状态；未 push，未创建 PR。
