# wgpu #8518：已初始化资源检查的快速路径实施方案

日期：2026-09-18

关联任务：[Optimize the initialization check for already initialized buffers and textures #8518](https://github.com/gfx-rs/wgpu/issues/8518)

状态：阶段 A 已完成；阶段 B 已实现并完成性能实验，性能验收未通过；C/D 尚未实施。实际改动、测试与 A/B 数据见 [阶段 A/B 实施记录](issue_8518_phase_ab_result.md)。下文保留设计与验收标准，不以已有实现替代验收。

实现提交：`3ba57ab26244044dcfe789add07b33df7b5d64b7`，分支 `codex/issue-8518-buffer-init`；作为实验候选存档，尚未 push。

## 1. 决策与实施范围

阶段 A/B 实测后，建议暂不直接合入当前 buffer 候选：共享 buffer 的并行编码有收益，但通用 compute 未达到 5% 目标，独立 buffer 的 2 线程场景有退化信号。先解决 B 的性能问题，再判断是否投入 texture。

本方案的设计是在保留现有细粒度初始化跟踪的基础上，对“整个资源已经初始化”的情况，以原子状态读取跳过初始化检查的读锁。

不把所有纹理直接改成一个可反复置位、清零的 `AtomicBool`。render attachment 的 `StoreOp::Discard` 会使已初始化状态失效，而且 command buffer 可以按不同于编码的顺序提交，需要单独证明其正确性。

分阶段交付如下：

| 阶段 | 交付内容 | 完成标准 |
| --- | --- | --- |
| A：基线与状态审计 | 热点、读写位置、初始化发布时序、基线数据 | 在固定版本上确认热点仍有优化价值；列清所有状态更新路径 |
| B：Buffer | 单向完成标志、封装后的查询和更新入口、回归测试 | 全初始化后跳过初始化读锁；未初始化和部分初始化语义不变；实际编码基准有可重复收益 |
| C：普通纹理 | 仅对不能进入 discard 状态的资源启用同类快速路径 | mip/layer 覆盖判断正确；不跳过 command buffer 内的 discard 修复；独立证明额外收益 |
| D：附件纹理评估 | 对可 discard 纹理作进一步热点与时序分析 | 默认保留现有路径；只有收益和正确性证据齐备才进入实现 |

B 可以独立完成并交付。B、C 完成也不应表述为“所有 texture 检查已无锁”，是否关闭 #8518 要结合实际覆盖范围和维护者意见。阶段 D 不作为前两阶段的强制依赖。

本方案不包含：公共 API/feature 变更、HAL 分配策略改写、shader workgroup 清零优化、纹理 aspect 跟踪重构、其他已存在的初始化缺陷修复，以及提交、推送或创建 PR 的实际操作。

## 2. 仓库位置与证据版本

文档保存在应用仓库的 `wgpu-in-app/examples/`，与现有 issue 分析和实施方案放在一起；仓库根目录的 `docs/` 被当前 `.gitignore` 忽略，因此不在那里保存本次交付。实际生产代码属于 wgpu 仓库的 `wgpu-core`，不属于 `app-surface` 或应用包装层。

本机位置：

- 应用及本方案：`/Users/lijinlei/Rust/wgpu-in-app`。
- 当前 wgpu checkout：`/Users/lijinlei/Rust/forks/wgpu`。
- 应用的 `Cargo.toml` 当前通过本地 path 依赖该 wgpu checkout。

以下使用的 wgpu 源码路径，除非特别说明，均相对于 wgpu 仓库根目录。

| 对象 | 固定版本 | 用途 |
| --- | --- | --- |
| wgpu 上游 trunk 快照 | `8b6acaf22fd11cc90ca81587e34baf040781b122` | 热点和核心设计依据 |
| 本地 wgpu HEAD | `f7c40b2cf96327b9d2dd2ffef73400528d2ec4f7` | 更新位置、测试和命令核对 |
| WebKit main 快照 | `3b56898b2f7201ab42276d6efbd8f6daf69c5f70` | Metal 实现对照 |
| Dawn main 快照 | `293b812672117108cfe87b726e6a325a10e8344a` | 初始化状态和提交时机对照 |
| WebGPU 规范源码 | `cd910cf650d05481b60bad2b44476caff974962f` | 初始化语义与时间线依据 |

本地与上游快照的以下九个文件逐字节一致：`init_tracker/{mod,buffer,texture}.rs`、`command/{pass,compute,memory_init}.rs`、`resource.rs`、`device/queue.rs`（均在 `wgpu-core/src/`），以及 `benches/benches/wgpu-benchmark/computepass.rs`。这不表示两个仓库版本整体相同。

实施前重新检查实际目标 checkout 的 `AGENTS.md`、HEAD、工作区和这些入口。当前应用工作区有其他未提交改动，应保留；本方案不要求修改应用依赖、Xcode 工程或现有示例。不能把本次上游快照等同于 Safari/Chrome 发布版。

## 3. 问题与收益边界

issue 作者观察到，compute pass 编码基准中有 10–20% 的耗时位于初始化跟踪的 `RwLock`。这是历史测量，尚未在本机复现；不是本方案承诺的收益，更不是应用帧率提升比例。[来源 S1](#s1)

当前共享的 render/compute 绑定处理函数 `flush_bindings_helper()` 遍历绑定资源，对 buffer 执行：

```rust
action.buffer.initialization_status.read().check_action(action)
```

对 texture，则进入 `CommandBufferTextureMemoryActions::register_init_action()`，读取同类初始化锁。资源长期复用时，未初始化范围通常已经为空，但要先拿锁才知道查询没有结果。[来源 S3](#s3)

目标是减少这部分 CPU 元数据访问成本。预期受益于大量资源绑定、较多 dispatch/draw、资源跨帧复用的路径。对于 GPU 执行占主导、资源刚创建或只有局部范围初始化的负载，收益可能很小。

不得通过以下方式获得表面收益：关闭零初始化、漏记待初始化 action、把清理成本移动到 submit 而不统计、预先清零原本不用的全部资源，或只测一个脱离真实调用路径的原子读取循环。

## 4. 必须保持的语义

WebGPU 要求资源的可观察行为符合初始化规则，同时允许实现利用应用写入来避免多余清零。规范特别指出，队列操作涉及的资源初始化状态与入队顺序有关，不能只依据命令编码时看到的状态决定最终行为。[来源 S2](#s2)

本任务采用以下约束：

1. 全初始化快速标志只是“无需再做这项初始化检查”的证据，不是 GPU 完成信号，也不代替资源有效性、映射状态、使用冲突、barrier 和 lifetime 检查。
2. 未初始化数据仍须在应用能够读取或部分写入前正确处理；不能因为 shader 声称写 storage buffer/texture，就认为会覆盖全部内容。
3. 一个范围初始化不代表整个资源初始化。buffer 的空洞、纹理的其他 mip/layer、D3 的整 mip 跟踪粒度都必须保留。
4. 用户编码但未提交的 clear/copy，不能直接发布资源级的“全初始化”标志。丢弃这类 encoder 后，其他命令仍须得到正确零初始化。
5. `queue.write_buffer/write_texture` 的 pending writes 和用户持有的未提交 encoder 不同：前者已由队列接管，现有实现可以据此更新 tracker。必须沿用并核对其提交先后关系。
6. 初始化 action 的省略不能省掉后续验证错误。无效资源、已销毁资源、已映射 buffer 等仍须按原路径报错。
7. 在提交准备阶段已经更新全局状态后，若失败，现有 device-loss 保护必须保持。不能让一个初始化失败但仍可继续使用的资源保留成功标志。

## 5. WebKit 与 Dawn 可借鉴的部分

### 5.1 WebKit

WebKit 普通 buffer 的创建通过 `Device::safeCreateBuffer()` 调用 Metal `newBufferWithLength:options:`。该 Metal API 提供清零分配保证，因此这条路径没有 wgpu 这种逐字节未初始化范围的查询负担。此策略与底层分配契约有关，不应作为本任务中跨后端改写 wgpu 的理由。[来源 S8](#s8)

纹理的 `m_clearedToZero` 按 mip 映射到 slice 集合；`previouslyCleared()` 做哈希查询。创建纹理相关 bind group 时调用 `clearTextureViewIfNeeded()`，可以提前安排清零。部分 `Load + Discard` 附件在 pass 结束时另开 clear encoder。其启发在于初始化时机与状态粒度，并不是可直接照搬的原子标志方案。[来源 S9](#s9)

### 5.2 Dawn

Dawn buffer 使用 `mIsDataInitialized`，`NeedsInitialization()` 先做布尔判断。Metal 后端对全覆盖写入直接设置初始化状态，对部分覆盖或首次读取则按需要清零。它不采用 wgpu 的任意字节范围跟踪。[来源 S10](#s10)

纹理通过 `std::vector<bool>` 按 aspect/layer/mip 记录状态；检查仍可能遍历多个子资源，discard 会把对应状态改回未初始化。Metal 后端在提交阶段 `FillCommands()` 中基于资源使用信息执行 lazy clear。Dawn 还有设备级同步，不能从字段是普通 `bool` 推导出整个 Dawn 无锁，更不能直接复制到 wgpu 的并行编码路径。[来源 S11](#s11)

### 5.3 采用的设计取舍

采用 Dawn 的“完成状态要便宜”的思路，同时保留 wgpu 现有细粒度 tracker。初始化工作的实际安排继续由 wgpu 原有的 queue、mapping 和 command memory init 路径负责。此轮不提前清零所有资源，也不迁移 WebKit 的 bind group 清理策略。

## 6. 阶段 A：建立基线并审计所有状态更新

### 6.1 固定比较对象

实施时选定一个基线 revision，记录完整 SHA、编译器版本、features、后端、适配器、系统版本和编译 profile。候选版本应只包含本任务修改。基准所需的新增场景必须同时用于基线和候选版本。

先运行现有 `Computepass Encoding` 基准并作 CPU 采样，确认初始化锁仍出现在主要调用栈中。性能测试使用优化后的 bench profile；debug/锁观察构建只用于定位，不能与优化构建混作 A/B 比较。

### 6.2 入口清单

| 类型 | 当前文件/入口 | 实施要求 |
| --- | --- | --- |
| 绑定检查 | `command/pass.rs::flush_bindings_helper` | 首要热点；buffer 与 texture 分别接入 |
| 间接 dispatch | `command/compute.rs` | 使用统一的 buffer 查询入口 |
| 顶点、索引、间接绘制、bundle 执行 | `command/render.rs`、`command/bundle.rs` | 审计创建 action 和重新检查 action 两类调用 |
| 拷贝、clear、query resolve | `command/transfer.rs`、`clear.rs`、`query.rs` | 保留 `MemoryInitKind` 和范围裁剪语义 |
| 光追资源 | `command/ray_tracing.rs` | 机械迁移统一入口；无对应硬件时说明运行覆盖限制 |
| Bind group 创建 | `device/resource.rs` | 已初始化资源可少生成 action；动态 offset 语义不能改变 |
| Buffer 创建/导入 | `device/resource.rs`、`resource.rs` | 覆盖普通、mapped-at-creation、HAL 导入和 invalid 对象 |
| CPU 映射 | `device/mod.rs::map_buffer` | 区分 `uninitialized()` 与 `drain()`，保留非一致性内存处理 |
| 队列写入 | `device/queue.rs` | 覆盖写 buffer、内部 clear、写 texture、外部图像写入 |
| 提交时初始化 | `command/memory_init.rs` | 发布完成标志的位置与现有 tracker 修改一致 |
| Texture 特殊路径 | `resource.rs::mark_externally_initialized`、surface present | 单独审计；第一轮纹理快速路径默认排除 surface |

不能仅通过搜索 `check_action` 完成审计。还要搜索所有 `initialization_status`、`drain`、`discard`、构造函数及对 `mips` 的直接修改。记录每个写入点持有的 queue/submission/map 锁，尤其是它是否已经承担后续执行初始化的责任。

## 7. 阶段 B：Buffer 的数据结构与接口

### 7.1 状态不变量

定义内部状态 `fully_initialized`，其含义是“现有初始化系统已不再需要为此 buffer 的跟踪范围安排初始化”，并不表示所有 GPU 命令已执行完成。

| 状态 | tracker | 快速标志 | 查询行为 |
| --- | --- | --- | --- |
| 尚未初始化/部分初始化 | 存在非空未初始化范围 | `false` | 拿原有读锁并精确查询 |
| tracker 已完成，但缓存尚未发布 | 不存在非空未初始化范围 | `false` | 允许；只损失性能 |
| 全部初始化且已发布 | 不存在非空未初始化范围 | `true` | 返回无 action，不拿初始化读锁 |
| 禁止状态 | 仍存在非空未初始化范围 | `true` | 会漏清零，必须由封装和测试排除 |

对普通 buffer，完成状态只允许 `false → true`。不得在同一对象上恢复为未初始化状态；将来若引入 discard/reuse/reset，需要重新设计此不变量，而不是直接复用快速路径。

### 7.2 内部封装

建议在 `init_tracker/buffer.rs` 增加 buffer 专用的锁与摘要封装，名称可在实现时调整，例如：

```rust
struct BufferInitState {
    fully_initialized: AtomicBool,
    tracker: RwLock<BufferInitTracker>,
}
```

- 原子类型沿用 `wgpu_sync::atomic::{AtomicBool, Ordering}` 的可移植入口。
- 锁仍使用 `crate::lock::RwLock` 和原来的 `BUFFER_INITIALIZATION_STATUS` rank。
- 不将这个封装提升为全项目的通用锁类型，也不重写底层锁实现。
- 增加查询 `create_action/check_action`；内部先加载原子标志，未命中才获取读锁。
- 增加受控的 tracker 更新入口，集中更新完成标志，防止调用方忘记发布。
- 映射等必须访问 tracker 的路径提供窄接口；不让普通调用方任意替换底层 tracker。

查询的逻辑示意如下，具体签名沿用现有 buffer/action 参数：

```text
create_action(buffer, range, kind):
    if fully_initialized.load(Relaxed):
        return None
    guard = tracker.read()
    return guard.create_action(buffer, range, kind)
```

没有必要把标志缓存到每个 bind group、pass 或 thread 上。第一版只维护资源自身的一份摘要，减少失效状态的数量。

### 7.3 发布完成状态

推荐使用一个内部更新闭包或等价的受控写接口：

```text
with_tracker_mut(operation):
    guard = tracker.write()
    result = operation(guard)
    // operation 内部借用 tracker 的迭代器均已结束
    if guard.is_fully_initialized():
        fully_initialized.store(true, Relaxed)
    return result
```

闭包不能把引用 tracker 的 guard/iterator 返回到外面；不能为了新接口额外收集一个原本不存在的 `Vec`。尽量保留原有迭代、收集和 HAL 命令安排方式。若闭包因 panic 退出，保持旧的 `false` 是允许的；不要求在析构时强行发布完成标志。

具体注意事项：

1. `InitTrackerDrain` 的范围删除有惰性行为。必须在相关 drain 被完整处理或按现有契约结束后检查剩余范围，不能在创建 drain 时就发布。部分消费的 iterator 也必须基于 tracker 的最终实际状态判断，不推测其会完成。
2. `InitTracker::new(0)` 当前可包含 `0..0` 哨兵。`is_fully_initialized()` 应判断“不存在非空未初始化范围”，不能简单把容器非空等同于未初始化。本任务无需为了这一点重写通用 tracker 的表示。
3. 普通 buffer 可能只跟踪 `tail_start` 之前的范围，尾部 padding 由创建路径处理。即使跟踪范围为空，也必须保留尾部清理和资源创建顺序。
4. `map_buffer()` 中非一致性、只读且不可 flush 的分支只查看 `uninitialized()`，不 drain tracker。不能因为 CPU 可见字节被补零就设置 GPU 全初始化标志。[来源 S5](#s5)
5. HAL 导入、mapped-at-creation、invalid 构造要分别初始化封装。invalid 对象即使没有待初始化范围，仍须由原有验证拒绝使用。
6. 快速标志只省查询，第一轮不利用它省掉任意写路径、HAL 清零、映射或 queue 操作。

## 8. 并发与内存序证明

`Relaxed` 是目标实现，不是因为 issue 提到了它就直接采用。它不提供其他内存访问的发布/获取顺序。[来源 S12](#s12)

采用它需要同时满足：

- 读到 `false` 时仍获取 tracker 锁，因此读取具体范围的同步不变。
- 读到 `true` 时不访问未加锁的范围容器、迭代器、HAL 指针或其他非原子元数据。
- 该证据为单向事实；旧的 `false` 只多一次查询，不会少一次初始化。
- 后续 GPU 使用的顺序由现有 submission/pending writes/map 同步建立，不依赖这个原子变量传递 CPU 数据或 GPU fence。

实现评审必须逐项完成以下时序证明：

| 写入来源 | 已核对的现有结构 | 必须验证的结果 |
| --- | --- | --- |
| 提交时 lazy init | `allocate_submission()` 持有 command-index 写锁；全局状态修改后失败会丢失设备 | 另一线程编码时可以看到完成证据，但其后续提交不能越过负责初始化的 submission |
| Queue buffer 写入 | 在 pending writes 中编码 copy，登记资源后 drain tracker | 后续提交/映射会正确处理 pending writes，不能先读取后写入 |
| 映射初始化 | `map_buffer()` 填充可见范围并按条件 flush；map 状态另有同步 | 使用快速标志不能绕过 mapped-state 检查；有效 GPU 使用发生在正确的 unmap/flush 之后 |
| 创建和 HAL 导入 | 构造时建立初始 tracker，遵守现有 unsafe 导入契约 | 对象暴露前必须已经满足原有初始化义务；零长度不跳过 padding 处理 |

现有 submission 序列化和失败保护是重要依据，但以上表格不是已经完成的形式化证明。实施时须把实际修改点代入，核对每个返回、错误和析构路径。[来源 S6](#s6)

若发现 reader 实际依赖被跳过读锁所建立的 happens-before，需要重新选择发布点或使用匹配的 Release/Acquire；如果问题是后续状态还能被撤销，更强内存序也不能自动解决。不能用“换成 SeqCst”替代对资源生命周期的证明。

测试至少控制三类交错：编码与初始化发布并行、先编码后初始化再提交、初始化命令编码后丢弃。使用 barrier/channel 控制线程步骤；随机压力测试只作为补充。

## 9. 阶段 C：不会被 discard 的普通纹理

### 9.1 准入条件

第一轮仅纳入经源码审计确认不会恢复未初始化状态的普通 native texture：

- 不具备 render attachment 或 transient attachment 使用能力。
- 不属于 surface texture，也不属于存在额外外部生命周期失效规则的对象。
- 所有现有写入路径只减少未初始化范围；没有合法路径可调用 discard/reset。

判定必须依据资源本身的能力和对象种类，不能仅检查当前 texture view 的 usage。一张能作为 render attachment 的纹理，即使这次只被采样，也不能视为永不 discard。若当前版本出现其他可撤销初始化的 native 扩展，应相应收紧条件。

第一版默认走保守路径，明确匹配到合格对象才启用。调试构建可在 discard 入口断言不属于单向完成模式；不能使用 `unsafe` 声明来跳过这个证明。

### 9.2 跟踪粒度与发布

`TextureInitTracker` 按 mip 和 array layer 跟踪；D3 不独立跟踪 depth slice。只有所有 mip tracker 均无非空未初始化范围时，才能发布整个纹理完成。

发布在现有 drain/update 完成之后，统一覆盖 queue writes、提交阶段清理和显式外部初始化入口。第一版可在写锁内检查所有 mip 是否完成，避免立刻增加原子计数、bitset 或分层缓存。如果实测表明这项扫描增加了不可接受的写入成本，再考虑维护未完成 mip 数；不能提前同时引入两套优化。

不得仅根据某次写入的尺寸等于纹理尺寸来设置完成：还需尊重 mip、layer、D3 深度和 aspect 的现有覆盖判定。本任务保持 wgpu 当前 aspect 初始化契约，不直接搬用 Dawn 的 aspect 位图。

### 9.3 保留 command buffer 内的修复

`register_init_action()` 有两个职责：检查全局 tracker、处理当前 command buffer 之前留下的 discard。只允许跳过前者：

```text
register_init_action(action):
    if not eligible_and_fully_initialized(action.texture):
        按现有逻辑查询 tracker，并追加必要的 init action
    始终执行原有 command-buffer-local discard 匹配和修复逻辑
    返回需要立即执行的清理
```

不能在函数顶部因为纹理已初始化就直接 `return`。[来源 S4](#s4)

在理论上阶段 C 的合格资源不应进入 discard 列表；仍保留后半段逻辑，可以使职责边界明确，避免未来扩展资格时意外跳过修复。

## 10. 阶段 D：可 discard 纹理的边界

此类纹理继续使用当前带锁路径。先测量阶段 B、C 后的剩余成本，再决定是否值得继续。

以下简单方案不能直接采用：在初始化时写 `true`，discard 时写 `false`，编码线程只读一次标志就永久省略 action。反例是：纹理已初始化；先编码读取命令 B；另一个命令 A 随后 discard；最终按 A、B 提交。B 编码时的 `true` 不足以证明它执行前的状态。

这个例子用于约束新设计，不是在本方案中宣告当前 wgpu 已存在一个已复现缺陷。若基线测试也失败，应先区分既有行为和候选改动，不顺带扩展修复范围。

完整扩展至少需要回答：

1. 当编码时省略了 action，提交阶段如何得知后续发生过影响该资源的 discard？
2. 是否必须保留提交时重检记录？如果需要，记录、引用计数和遍历成本是否抵消收益？
3. 版本号/epoch 如何与提交顺序关联？单独加载版本号无法防止“检查之后、提交之前”发生 discard。
4. D3 的 depth-slice 修复、depth/stencil 的现有语义和多 command buffer 提交是否保持？
5. 对重用 render bundle，是否会跨提交保留过期的初始化判断？

没有上述答案时，不扩大阶段 C 的资源资格。这是一项明确的覆盖限制，不以未测量的“全纹理优化”作为交付目标。

## 11. 改动文件与实施顺序

| 顺序 | 文件 | 计划改动 | 检查重点 |
| --- | --- | --- | --- |
| 1 | `wgpu-core/src/init_tracker/mod.rs` | 增加无非空待初始化范围的查询能力 | `new(0)`、空范围、drain 生命周期；不改范围算法 |
| 2 | `wgpu-core/src/init_tracker/buffer.rs` | Buffer 状态封装和快速查询 | 单向标志、锁 rank、原子可移植性 |
| 3 | `wgpu-core/src/resource.rs`、`device/resource.rs` | 替换 buffer 状态字段与构造入口 | 普通、invalid、HAL 导入、mapped-at-creation、尾部清理 |
| 4 | `wgpu-core/src/device/mod.rs`、`device/queue.rs`、`command/memory_init.rs` | 将 tracker 更新接入统一发布位置 | CPU/GPU 初始化责任、错误路径、迭代器完成后发布 |
| 5 | `wgpu-core/src/command/{pass,compute,render,bundle,transfer,clear,query,ray_tracing}.rs` | 迁移 buffer 的 create/check action 调用 | 不改变验证顺序、action kind、范围或资源引用关系 |
| 6 | `tests/tests/wgpu-gpu/zero_init.rs`、相关内部测试 | 增补确有缺口的回归 | 优先复用已有辅助代码，避免复制整套测试框架 |
| 7 | `benches/benches/wgpu-benchmark/computepass.rs` 或独立同级场景 | 增加能区分资源初始化状态的基准 | 比较器两侧使用同样场景和计时边界 |
| 8 | `init_tracker/texture.rs` 及纹理读写位置 | 阶段 C 的资格、完成摘要和快速查询 | 只优化合格资源；保留所有 discard 修复 |

先完成 B 的生产代码、正确性验证和 A/B 测量，再进行 C。若 B 没有可重复收益，先查热点比例、缓存布局和测试场景，不自动增加更复杂的纹理缓存。

不在本方案中预定新的通用 trait、原子 bitset、后台任务、额外每帧扫描或新的 Cargo feature。需要的抽象以保护上述状态不变量为限。

## 12. 正确性验证矩阵

### 12.1 内部状态和并发测试

| 用例 | 构造 | 断言 |
| --- | --- | --- |
| 零长度 | `new(0)` 和空查询范围 | 无有效未初始化范围；构造和查询语义保持 |
| 全新资源 | 非零完整待初始化范围 | 不能命中快速完成路径 |
| 部分初始化 | 先初始化中间范围，再查询前、中、后 | 空洞仍被跟踪，完整标志保持 false |
| 多段完成 | 分多次初始化互不相邻范围，最后填满空洞 | 只在最后一步允许发布 true；已有数据不被再次清零 |
| drain 生命周期 | 完整消费、不消费即结束、部分消费后结束 | 摘要始终来自 tracker 的实际结果；不能提前为 true |
| stale false | tracker 已完成，但摘要保守地仍为 false | 慢路径返回正确结果，不产生误清零 |
| 并发编码/更新 | 两个线程通过 barrier 控制更新前后查询 | 合法结果与原 tracker 一致；不存在未初始化时跳过 action |
| 资格回退 | 附件纹理、surface、不满足资格的对象 | 即使当前全初始化，也继续使用原检查路径 |

若要证明热路径没有获取该读锁，可使用测试专用的锁观察/计数或现有 `observe_locks` 能力。不要在正式查询路径加入为了测量而存在的全局原子计数器，也不要用可能永久死锁的测试来推测是否加锁。

若依赖单纯线程压力仍难覆盖发布边界，可补充最小并发模型验证；不为了本任务默认引入全仓库的新测试框架。并发模型不能代替真实 queue/readback 验证。

### 12.2 Buffer GPU 行为

| 场景 | 执行顺序 | 必须验证 |
| --- | --- | --- |
| 全新读取 | 创建 buffer → copy/map 或 shader 读取 | 可观察未写字节为零 |
| 局部写入 | 写入带非零哨兵的中间区域 → 读取完整资源 | 写入区保留，前后未写区为零 |
| 完整写入复用 | 写满 → 多轮 compute/render 使用 | 完成后不再因初始化逻辑清零已有值 |
| Mapped-at-creation | 映射创建，修改部分字节，unmap，再使用 | 修改值保留，其余遵守零初始化语义 |
| Map 范围/一致性 | 分段映射，以及可用后端的非一致性只读路径 | 不把仅 CPU 可见的补零误判为 GPU 全初始化 |
| 未提交初始化 | 编码 clear/copy 的 encoder A 后丢弃，再提交读取 B | B 仍正确触发初始化 |
| 编码/提交顺序不同 | 先编码读取 B，再进行队列写入或提交初始化 A，最后提交 B | 读取正确数据，无多余清零破坏写入 |
| 并行编码 | 多线程共享已初始化 buffer，按确定顺序提交 | 结果一致；保留资源验证和使用同步 |
| 动态 offset 与 bundle | 部分范围、非零 offset、重复执行 bundle | 原范围语义保持，未覆盖部分不被忽略 |
| 边界与验证 | 零长度、非 4 字节可见长度、tail padding、invalid/destroyed/mapped | 不越界、不跳过错误、不漏尾部初始化 |

先复用 `zero_init.rs` 已有的 dynamic offset、buffer tail、texture-to-buffer padding、零大小 binding 等用例，只对快速标志带来的新增风险补测试。[来源 S7](#s7)

新分配的底层内存可能恰好为零；“读回零”单独不足以证明 lazy init 被执行。应结合内部状态断言，并在具备条件的 Vulkan/llvmpipe 环境用 `LVP_POISON_MEMORY=true` 加强验证。Apple Metal 的分配保证可能掩盖漏初始化，不能以本机通过代替跨后端证明。

### 12.3 Texture GPU 行为

| 场景 | 覆盖要求 |
| --- | --- |
| 仅采样或 storage 的普通纹理 | 全初始化后反复绑定，验证命中阶段 C 快速路径并保持内容 |
| 多 mip/多 layer | 仅写一部分；读取其他子资源必须正确初始化；最后一块完成才可发布全完成 |
| 部分区域写入 | `write_texture`、buffer-to-texture、texture-to-texture 的覆盖判断保持 |
| D3 | 写一个 depth slice 不等同于初始化所有 slice；沿用整 mip 的初始化粒度 |
| Depth/stencil 与其他格式 | 复用当前测试覆盖；不改变既有 aspect 处理，不因摘要跳过保留数据的检查 |
| 同 command buffer discard 后读取 | 即使纹理之前已经初始化，仍正确执行局部修复 |
| 跨 command buffer discard | 同一次 submit 的多 command buffer、分次 submit、编码顺序与提交顺序不同 |
| 外部初始化和 surface | 保留 `mark_externally_initialized` 与 present 清理；不意外启用阶段 C 快速路径 |

附件纹理测试在 C 中主要验证“仍正确回退”，不能把这些测试通过解读为附件纹理已经得到优化。硬件/feature 不支持的用例记录为未覆盖或 skip，不能写为 pass。

### 12.4 建议运行命令

以下是实施后的命令模板，本次文档工作没有执行这些构建或测试。先在实际 wgpu checkout 核对 `AGENTS.md`、`docs/testing.md`、工具依赖和筛选参数。

```sh
# 从实际 wgpu checkout 根目录执行
cargo fmt --check
cargo clippy --tests
cargo test -p wgpu-core --lib init_tracker

# 查看并运行相关 GPU 测试
cargo xtask test --list --test wgpu-gpu zero_init
WGPU_BACKEND=metal cargo xtask test --test wgpu-gpu zero_init

# 初始化 CTS；buffer 阶段重点跑 buffer，纹理阶段增加 texture_zero
cargo xtask cts --backend metal 'webgpu:api,operation,resource_init,buffer:*'
cargo xtask cts --backend metal 'webgpu:api,operation,resource_init,texture_zero:*'

# 完成前按目标 checkout 契约执行所需的整体回归
cargo xtask test
cargo xtask cts --backend metal
```

若启用 `wgpu_validate_locks` 或 `observe_locks` 检查锁行为，使用独立构建和结果目录，不把此构建的耗时放进性能结论。对现有失败记录基线/候选是否相同，不顺带修复无关问题，也不为了让结果变绿而扩大 skip 列表。

## 13. 性能验证方法

### 13.1 场景

现有 `Computepass Encoding` 基准默认包含 10,000 次 dispatch、不同 bind group 和单/多线程编码；计时分别记录 encoding 与 submit。它是主验收入口，不能用孤立的原子微基准替代。[来源 S7](#s7)

至少增加或明确区分以下场景：

| 场景 | 资源状态 | 目的 |
| --- | --- | --- |
| 稳定复用 | 测量前完成初始化并等待；测量时反复使用 | 测快速路径的主要收益 |
| 首次使用 | 每轮使用新的未初始化资源 | 检查新增分支与发布操作的成本 |
| 长期局部初始化 | 热区已写、资源其他部分保持未初始化 | 检查 fast flag 长期为 false 时的退化；代表大 buffer 分区使用 |
| 多线程 | 2/4/8 个编码线程，区分共享资源和独立资源 | 测锁竞争与无竞争情况，不能假设并行必然加速 |
| Render 回归 | 现有 renderpass 编码与 bundle 路径 | 共享 helper 变化不能只验证 compute |
| 创建与队列写入 | buffer 创建、完整/部分写入的固定工作量 | 观察写侧摘要更新是否抵消读侧收益 |

Buffer 阶段、texture 阶段分别记录增量效果。不要把混合场景的全部收益都归因于 buffer；当前 compute 场景同时涉及多类资源。

### 13.2 计时与比较

1. 基线和候选使用同一硬件、系统、Rust 工具链、features、后端和 bench profile；正式计时关闭 Metal validation 和 profiler。
2. 稳定复用场景显式完成 warm-up 初始化及必要等待，放在计时区间外；首次使用场景则把该项作为被测工作明确记录。
3. 一轮输出至少包含 encoding、submit 和两者合计；另列 GPU 等待时间，不混作 CPU 编码时间。
4. 初始采用至少 5 组交替 A/B 独立进程运行，记录中位数和离散度；样本不稳定时先消除环境干扰，再决定是否需要补测。
5. 读锁调用数/采样占比和端到端耗时相互印证。只有锁调用减少，而总耗时未改善，不能宣称达成性能目标。
6. 记录 `Buffer`/`Texture` 尺寸和对齐变化；一个布尔字段可能带来 padding，必要时评估大量资源下的内存成本。

建议在实施前固定项目内部验收门槛：主目标场景的 CPU encoding 中位数下降至少 5%，且明显超过 A/B 波动；encoding+submit 合计有收益，冷路径和部分初始化场景无超过噪声的实质退化。这是本方案建议的投入判断线，不是上游要求，也不是预测结果。若只获得较小但稳定收益，应结合 diff 复杂度单独作是否继续的决策，不能修改口径把它写成已达到原目标。

### 13.3 基准命令和结果文件

```sh
# 在基线 checkout 执行；--time 的支持已按当前 benchmark CLI 核对
WGPU_BACKEND=metal cargo bench -p wgpu-benchmark -- \
  'Computepass Encoding' --time 5 --save-baseline issue-8518-base-compute-r1

# 在候选 checkout 执行；比较前先将上面的 baseline JSON 放入其 target/bench
WGPU_BACKEND=metal cargo bench -p wgpu-benchmark -- \
  'Computepass Encoding' --time 5 \
  --baseline issue-8518-base-compute-r1 --save-baseline issue-8518-buffer-compute-r1

# Render 的结果必须单独命名，避免覆盖 compute 基线
WGPU_BACKEND=metal cargo bench -p wgpu-benchmark -- \
  'Renderpass Encoding' --time 5 --save-baseline issue-8518-base-render-r1
```

后续各轮递增 `r1`，候选 render 运行也使用自己的 `--baseline` 和 `--save-baseline` 名称。新增的独立场景使用其实际注册名筛选。

当前基准把 JSON 写到由 `CARGO_MANIFEST_DIR` 决定的仓库 `target/bench/`，不是仅设置共同的 `CARGO_TARGET_DIR` 就能共享比较文件。多个 checkout 之间需要显式复制所需 JSON。原始日志、采样和临时脚本放在忽略的 `target/issue-8518/` 或仓库外，最终报告只保留必要结果与复现信息。

### 13.4 结果表模板

| Revision/阶段 | 场景 | 线程数 | Encoding 中位数 | Submit 中位数 | 合计中位数 | 波动/样本数 | 结论 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 基线 | 稳定复用 | 1 | 待测 | 待测 | 待测 | 待测 | 待测 |
| B：Buffer | 稳定复用 | 1 | 待测 | 待测 | 待测 | 待测 | 待测 |
| B+C | 稳定复用 | 1 | 待测 | 待测 | 待测 | 待测 | 待测 |
| 基线/候选 | 首次使用、部分初始化、2/4/8 线程、render | 按场景 | 待测 | 待测 | 待测 | 待测 | 待测 |

## 14. 里程碑、停止条件与交付检查

### M0：是否继续投入

完成当前基线和热点采样。如果初始化检查在目标负载中已不再显著，应记录原因并停止扩大实现；可以保留此方案，但不能以历史 10–20% 代替当前证据。

### M1：Buffer 可独立验收

所有 buffer 读路径使用统一接口，所有初始化写路径覆盖到完成标志，正确性测试通过，并获得可重复 A/B 数据。若出现新增失败，先修复或缩小本阶段改动，再进行纹理工作。

### M2：普通纹理的增量价值

资格审计完成，mip/layer 完成判定正确，discard/surface 路径保持回退，且在 B 的基础上有可重复增益。如果新增复杂度大于收益，只交付 B，并明确纹理尚未优化。

### M3：提交准备状态

- 改动归属集中在 `wgpu-core`；没有以应用层规避代替 core 优化。
- 文档中的全部“已验证”都能对应实际命令、revision 和结果；未运行的平台明确标注。
- 不再从 reader 无锁访问 tracker 内部数据；没有通过更新摘要引入新的锁顺序。
- 有确定性的未初始化/部分初始化/编码与提交顺序回归；GPU 回读没有被“底层分配碰巧为零”单独支撑。
- 有 buffer 单独与纹理增量的性能数据；CPU 编码、提交、GPU 执行的收益边界清楚。
- 根据实际提升及仓库政策判断是否需要 changelog；不预写没有测量支撑的百分比。
- 必要检查已通过且没有新风险证据时停止扩测。

达到 M3 表示实现可供审阅，不等同于已提交、已推送、PR 已被接受或 issue 已解决。实际 Git/PR 操作仍按后续用户请求和目标仓库约束执行。

## 15. 风险及处理方式

| 风险 | 后果 | 处理 |
| --- | --- | --- |
| 过早发布 true | 漏清零，可能暴露未初始化数据 | 统一写入口；逐路径证明提交/映射顺序；确定性交错测试 |
| 漏迁移一个写入口 | 通常保持 false，优化没有覆盖；也可能产生不一致 | 搜索所有构造和修改位置；摘要封装限制直接访问 |
| 零长度哨兵误判 | 导入或零长度对象无法命中，或 padding 漏处理 | 区分逻辑空范围和容器空；保留原尾部流程 |
| 把局部初始化当全完成 | 未写区域、其他 mip/layer 被跳过 | 摘要从完整 tracker 推导，不根据一次写入猜测 |
| 将附件纹理纳入单向状态 | discard 后读取错误 | 默认保守资格；资源级判断；单独阶段 D |
| 绕过局部 discard 修复 | 同 command buffer 中清理顺序错误 | 仅跳过全局 tracker 查询，不跳过整个注册函数 |
| 原子字段/写侧扫描增加成本 | 冷资源或大量资源创建退化 | 冷路径基准、结构体尺寸记录；不提前上复杂计数器 |
| 只在 Metal 验证 | 其他内存模型/后端路径缺覆盖 | 共用测试加其他后端验证，无法运行时明确限制 |
| 基准差异来自环境或成本转移 | 错误收益结论 | 固定配置、交替 A/B、分别报告 encoding/submit |

## 16. 证据索引

以下源码链接固定 revision，方便实施时区分本方案依据与后来上游变动；行号用于定位，函数名和实际代码是最终依据。

<a id="s1"></a>
**S1：问题与原始热点报告**

- [wgpu #8518](https://github.com/gfx-rs/wgpu/issues/8518)。本次查询时为 Open；作者报告 10–20% 的 compute 编码时间与初始化锁有关。

<a id="s2"></a>
**S2：WebGPU 规范**

- [固定规范：Uninitialized data](https://github.com/gpuweb/gpuweb/blob/cd910cf650d05481b60bad2b44476caff974962f/spec/sections/privacy-and-security.bs#L38-L56)。
- [固定规范：buffer 分配规则](https://github.com/gpuweb/gpuweb/blob/cd910cf650d05481b60bad2b44476caff974962f/spec/index.bs#L3691)。

<a id="s3"></a>
**S3：wgpu 读取热点与跟踪粒度**

- [共享绑定检查](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/command/pass.rs#L204-L235)。
- [Buffer action 的创建与检查](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/init_tracker/buffer.rs)。
- [Tracker 范围、drain 和零长度构造](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/init_tracker/mod.rs#L1-L259)。
- [Texture mip/layer 粒度](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/init_tracker/texture.rs)。

<a id="s4"></a>
**S4：wgpu 初始化 action 与 discard**

- [register_init_action 及局部 discard 修复](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/command/memory_init.rs#L80-L162)。
- [Buffer 提交阶段初始化](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/command/memory_init.rs#L225-L283)。
- [Texture 提交阶段初始化及 D3 处理](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/command/memory_init.rs#L333-L444)。

<a id="s5"></a>
**S5：构造、映射和队列写入**

- [本地映射初始化及 non-coherent 分支](/Users/lijinlei/Rust/forks/wgpu/wgpu-core/src/device/mod.rs:264)。
- [本地 buffer 创建、跟踪范围与 padding](/Users/lijinlei/Rust/forks/wgpu/wgpu-core/src/device/resource.rs:1240)。
- [本地 HAL buffer 导入](/Users/lijinlei/Rust/forks/wgpu/wgpu-core/src/device/resource.rs:1630)。
- [Queue 写入后更新 buffer tracker](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/device/queue.rs#L938-L963)。
- [外部 texture 初始化入口](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/resource.rs#L2269-L2281)。

<a id="s6"></a>
**S6：提交顺序与失败保护**

- [更新全局状态前的验证与失败保护](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/device/queue.rs#L1511-L1655)。
- [失败时 device loss](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/device/queue.rs#L1760-L1773)。
- [allocate_submission 的 command-index 锁](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/wgpu-core/src/device/queue.rs#L1795-L1840)。

<a id="s7"></a>
**S7：现有测试、基准和 CLI**

- [本地 zero-init GPU 测试](/Users/lijinlei/Rust/forks/wgpu/tests/tests/wgpu-gpu/zero_init.rs:1)。
- [现有 compute 编码基准](https://github.com/gfx-rs/wgpu/blob/8b6acaf22fd11cc90ca81587e34baf040781b122/benches/benches/wgpu-benchmark/computepass.rs#L440-L552)。
- [本地 benchmark 使用说明](/Users/lijinlei/Rust/forks/wgpu/benches/README.md:1)。
- [本地 benchmark CLI](/Users/lijinlei/Rust/forks/wgpu/benches/src/lib.rs:90) 与 [结果目录实现](/Users/lijinlei/Rust/forks/wgpu/benches/src/file.rs:5)。
- [本地测试契约](/Users/lijinlei/Rust/forks/wgpu/docs/testing.md:1)、[buffer CTS](/Users/lijinlei/Rust/forks/wgpu/cts/src/webgpu/api/operation/resource_init/buffer.spec.ts:1)、[texture CTS](/Users/lijinlei/Rust/forks/wgpu/cts/src/webgpu/api/operation/resource_init/texture_zero.spec.ts:1)。

<a id="s8"></a>
**S8：WebKit buffer 与 Metal 分配契约**

- [WebKit safeCreateBuffer 与普通 buffer 创建](https://github.com/WebKit/WebKit/blob/3b56898b2f7201ab42276d6efbd8f6daf69c5f70/Source/WebGPU/WebGPU/Buffer.mm#L125-L180)。
- [Apple makeBuffer(length:options:) 文档](https://developer.apple.com/documentation/metal/mtldevice/makebuffer%28length%3Aoptions%3A%29)。

<a id="s9"></a>
**S9：WebKit texture 初始化**

- [Texture 清理状态](https://github.com/WebKit/WebKit/blob/3b56898b2f7201ab42276d6efbd8f6daf69c5f70/Source/WebGPU/WebGPU/Texture.mm#L4151-L4200)。
- [Bind group 创建时的清理入口](https://github.com/WebKit/WebKit/blob/3b56898b2f7201ab42276d6efbd8f6daf69c5f70/Source/WebGPU/WebGPU/BindGroup.mm#L1143-L1148)。
- [Queue 遍历待清理子资源](https://github.com/WebKit/WebKit/blob/3b56898b2f7201ab42276d6efbd8f6daf69c5f70/Source/WebGPU/WebGPU/Queue.mm#L2107-L2138)。
- [Discard 清理条件](https://github.com/WebKit/WebKit/blob/3b56898b2f7201ab42276d6efbd8f6daf69c5f70/Source/WebGPU/WebGPU/RenderPassEncoder.mm#L150-L203) 与 [pass 结束补清理](https://github.com/WebKit/WebKit/blob/3b56898b2f7201ab42276d6efbd8f6daf69c5f70/Source/WebGPU/WebGPU/RenderPassEncoder.mm#L1335-L1345)。

<a id="s10"></a>
**S10：Dawn buffer**

- [NeedsInitialization](https://github.com/google/dawn/blob/293b812672117108cfe87b726e6a325a10e8344a/src/dawn/native/Buffer.cpp#L1120-L1122)。
- [Metal 完整覆盖、部分覆盖和清零](https://github.com/google/dawn/blob/293b812672117108cfe87b726e6a325a10e8344a/src/dawn/native/metal/BufferMTL.mm#L247-L294)。

<a id="s11"></a>
**S11：Dawn texture、提交与同步**

- [子资源状态查询和更新](https://github.com/google/dawn/blob/293b812672117108cfe87b726e6a325a10e8344a/src/dawn/native/Texture.cpp#L1402-L1439)。
- [Discard 恢复未初始化](https://github.com/google/dawn/blob/293b812672117108cfe87b726e6a325a10e8344a/src/dawn/native/CommandBuffer.cpp#L227-L234)。
- [Metal 提交时 lazy clear](https://github.com/google/dawn/blob/293b812672117108cfe87b726e6a325a10e8344a/src/dawn/native/metal/CommandBufferMTL.mm#L1087-L1133)。
- [设备级同步实现](https://github.com/google/dawn/blob/293b812672117108cfe87b726e6a325a10e8344a/src/dawn/native/DeviceGuard.cpp#L49-L95)。

<a id="s12"></a>
**S12：内存序与锁实现**

- [Rust Ordering::Relaxed](https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html#variant.Relaxed)。
- [本地 wgpu 原子类型可移植入口](/Users/lijinlei/Rust/forks/wgpu/wgpu-sync/src/atomic.rs:1)。
- [本地可观察/分级锁选择](/Users/lijinlei/Rust/forks/wgpu/wgpu-core/src/lock/mod.rs:1)。
