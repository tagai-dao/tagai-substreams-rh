# RH V14 索引开发方法与服务器交接记录

记录日期：2026-10-09。最新观测日志为 `2026-10-09T09:41:14+02:00`，
即北京时间 **2026-10-09 15:41:14**。这是操作员回传的状态快照，不是实时监控。
下次开发或操作服务器前，必须重新读取实际配置、包身份和数据库游标。

## 1. 下次从这里开始

先读本文件了解本次结果，再读 [操作规范](SUBSTREAMS_AGENT_RUNBOOK.md)。
V14 事件、字段和兼容性细节见 [RH_V14_COMPATIBILITY.md](RH_V14_COMPATIBILITY.md)，
V11、Basket、Nutbox 和 IndexBroker 的旧语义见
[RH_V11_COMPATIBILITY.md](RH_V11_COMPATIBILITY.md)。精确包、备份及验证证据在
[`v14-v0.6.1-server-candidate.json`](../deployments/4663/v14-v0.6.1-server-candidate.json)。
该文件名保留了 candidate 字样，但其中已经记录实际部署和运行证据。

当前状态：**V14 统一索引已切换并运行，正在追赶历史；尚未到达 V14 部署区间。
PostgreSQL 的 V14 事件级验收、MySQL 投影和 API 验收仍未完成。**

协作规则长期有效：

- 本地开发、构建、检查由 Codex 完成。服务器调试和部署每次只给操作员一个命令，
  说明预期结果；操作员执行并回传后，再给下一步。不要直接连接服务器代为操作。
- 服务器 PostgreSQL 查询和迁移给可执行的服务器命令。迁移先准备、审查 SQL 文件，
  说明目标数据库和影响，再由操作员执行。
- TagAI MySQL 迁移给完整 SQL，由操作员直接在数据库执行。
- 开发和构建期间保持生产索引运行；切换时才在记录过的边界停旧写入者。
- 发布验证包括 PostgreSQL、MySQL、API、续跑和回滚，不能把部署成功当作产品验收完成。

## 2. 已知服务器状态

| 项目 | 本次确认值 |
| --- | --- |
| 主机 | `vmi3433492`，操作员使用 root |
| 链 | Robinhood，chain ID `4663` |
| 本地仓库 | `/Volumes/Extreme Pro/wangxi/work/tiptag/tiptag-substreams`，`main` |
| Git 远端 | `git@github.com:tagai-dao/tagai-substreams-rh.git` |
| 服务器源码目录 | `/root/tagai-substreams-rh` |
| 运行目录 | `/opt/tiptag-substreams`，不是 Git 仓库 |
| SQL sink | `/opt/tiptag-substreams/bin/substreams-sink-sql` |
| 入口 | `/opt/tiptag-substreams/scripts/run-incremental-sink.sh` → `run-sink.sh` |
| systemd | `tiptag-unified-incremental.timer` / `.service` |
| 数据库容器 | `tiptag-substreams-postgres`，`postgres:17` |
| 当前数据库 | `tiptag_rh`；另有 `postgres`、`tiptag_rh_v11`，不要据名称改目标库 |
| 服务环境文件顺序 | `/etc/tiptag-unified-substreams.env`，再 `/opt/tiptag-substreams/.substreams.env` |
| Pinax 端点 | `robinhood.substreams.pinax.network:443` |
| 模式 / worker | Production；免费版上限 **5 个 worker**，操作员已确认 |
| 实际启动超时 | `TimeoutStartUSec=12h`，停止超时 90 秒 |

`.substreams.env` 含凭证，启动脚本还会再次加载它；只输出需要核对的非敏感配置。
仓库 unit 模板的 2 小时超时与本次服务器实际 12 小时不同，不能用模板推断线上值。
服务器最近明确拉取的索引源码为 `7ff3065`；此后文档和 launcher 提交是否已拉取，
没有操作员证据，必须重新查询。当前运行 SPKG 的源码身份仍为 `7ff3065`。

当前已确认配置：

```dotenv
PACKAGE_PATH=/opt/tiptag-substreams/tiptag-v14-continuation-v0.6.1.spkg
START_BLOCK=78241606
MODULE_HASH_MISMATCH_POLICY=error
CURSORS_TABLE=unified_v14_cursors
HISTORY_TABLE=unified_v14_substreams_history
INCREMENTAL_CURSOR_ID=17a9486293f4cf6aaeb1b2e02724bae301332352
INCREMENTAL_MAX_BLOCKS=300000
INCREMENTAL_MAX_BLOCKS_CEILING=8000000
LATEST_LAG_BLOCKS=6000
```

正常有进展后，下一轮恢复 30 万基础窗口；成功但游标不变时按 2 倍扩大到最多 800 万。
状态文件为 `/var/lib/tiptag-substreams/incremental-window.state`。
`START_BLOCK` 是首次启动/零游标时的边界；非零游标后从已提交位置加一接续。

### 最新进度证据

| 信号 | 时间（服务器 +02:00） | 高度 / 数值 |
| --- | --- | --- |
| 已提交游标，来自 wrapper 的数据库读取 | 08:17:54 | **81,489,309**；最新 block ID 未回传 |
| 当前轮次开始 | 08:17:55 | 起点 **81,489,310**，目标 **81,789,309**，stop exclusive **81,789,310** |
| 当前轮启动时链目标 | 08:17:55 | 链头 **83,939,902**，减 6,000 后目标 **83,933,902** |
| 最后收到的输出块 | 09:41:14 | **81,714,795**；不是数据库提交证明 |
| 最后收到的输出块 hash | 09:41:14 | `3c145603e919942281a2547d323ea1557844f5eb18b4202edd10105d8e4891dc` |
| 远端阶段 | 09:41:14 | stage 0–4：81,722,000 / 81,719,000 / 81,718,000 / 81,712,000 / 81,714,000 |
| 远端任务 / 处理计数 | 09:41:14 | 5 个任务；`progress_total_processed_blocks=4005` |

该轮 wrapper PID 为 `36680`，sink PID 为 `36702`；PID 只对应这轮日志，下一轮会变。
从最终切换游标 78,241,605 到最新确认游标，累计推进 **3,247,704 块**。
距 V14 最早部署高度 **83,024,792** 仍差 **1,535,483 块**。
09:41 的 stream 日志尚没有本轮完成记录，也没有重新查询数据库，因此不要将
81,714,795 或 stage 高度记成新的已提交游标。V14 当前表行数也未重新查询。

## 3. 本次采用的方法

原则是保留旧包和已经索引的数据，新功能拥有独立模块及部署起点，再合成统一输出。
下次先确定新功能最早事件块 H，与**停旧写入者后重新读取的数据库边界 C**比较。

| 条件 | 方案 |
| --- | --- |
| C < H，旧模块可完全保留，新域没有写过 SQL 历史 | 从 C+1 直接运行统一 continuation；随后自然经过新部署区间 |
| C >= H，新域已有需要补齐的历史，旧模块可保留 | 精确旧包 continuation + 新域历史 backfill，验证共同边界后统一接续 |
| 旧有状态、过滤器、聚合语义无法兼容 | 单独 PostgreSQL 数据库做 blue/green；按规范验证后切换 |

本次选第一种：最终旧游标 **C=78,241,605**，早于最早 Router 部署
**H=83,024,792**。Pump 部署块为 **83,065,677**。起点必须涵盖 Router 的初始化，
不能只用 Pump 部署块，否则会漏掉路由和价格池。

切换顺序实际执行如下：

1. 保持旧服务运行，开发 V14 独立模块；对照 BSC 映射核对语义及链差异。
2. 测试并构建模板，在服务器用**精确已安装 V0.5.3 SPKG**组装 continuation。
3. 审计 64 个 shared module hash：64 不变、0 改变，新增 10 个模块。
4. 对 Pump 部署块运行 store-free 静态解码探针，确认 52 条批准事件和 52 个资产。
5. 备份，执行 PostgreSQL additive schema 迁移；确认 11 张新表都有主键。
6. 用 SQL sink `setup --system-tables-only` 创建新 cursor/history 表。
7. 停 timer，再停旧 service，读取最终 C、block ID 和新域未写入的条件。
8. 对停稳的数据库和配置再备份；设置新包、C+1、新系统表和严格 hash policy。
9. 先跑 1,000 块 canary；无事件，成功，新的 output hash 初始化游标为 0。
10. 恢复自适应追赶；验证首次非零提交、下一轮从游标加一自动接续。
11. 追赶期间将基础窗口由 10 万改为 30 万；当前继续运行，后续验收仍待完成。

没有复制旧 opaque cursor，没有执行单独 V14 SQL backfill，没有重放已入库的旧历史。
新 cursor 表在旧数据库可用，是因为这是同一 canonical 数据集的非重叠接续；
不能把它推广成“新 cursor 表就能隔离任意重放”。

## 4. 开发和打包时必须保留的经验

- `substreams.yaml` 是开发模板，不是可直接部署的生产 continuation。
  旧 module 定义、WASM、filter、network override 都从精确生产 SPKG 复制。
  相同源码或相同文件名，不代表相同 module hash。
- `examples/make_v14_continuation.rs` 将旧包实际 sink 绑定到统一输出，保留旧输出
  的有效 initial block（含 network override）。本次为 **53,869,281**。
  只有新域保留部署起点；把整个统一输出设到新部署块会漏掉 C+1 到 H 的旧事件。
- V0.6.0 请求曾估算约 1.63 亿 stage-block，超过默认 10,000 限额并在处理前拒绝。
  这不是“新合约实际要读 1.63 亿唯一链块”，也没有因此写入 SQL。
  小输出窗口不能限制上游 store 的历史准备范围，不要先调成无限额。
- V0.6.1 将旧 Walnut token/owner 元数据读取移到 stateless SQL output；
  V14 static map 和新 stores 从新域事件构建状态。移除 V14 Basket 不需要的旧 store 输入。
  完整 SQL 输出仍依赖旧 Walnut 读取状态，不能宣称所有历史准备都消失。
- 动态地址用 factory discovery store；处理同一交易创建、子合约事件、全局 ordinal、
  重复 log 和 undo。静态地址精确过滤；公共 Swap/Transfer 不能做链级触发过滤。
- 每个 module 最多 30 个直接输入。组件权重、staking reward ratio 和不同池类型
  不混用；TradeCuration membership 用 union view，不能重复累加旧 users_count。
- Holder 快照保持 Blockscout → `tiptag-server` → MySQL，不加入 Substreams Transfer 索引。
- 验证命令包括 `cargo test`、`substreams build`、`substreams info`、SHA-256、
  shared hash audit、每个新事件族的有界实链回放及数据库、下游和恢复验证。
  本次本地 40 个 library tests、2 个 assembler tests 通过；不能用它替代未完成的实链验收。
- CLI JSONL 末尾可能含 `Completed successfully`。只跳过空行和明确 footer，
  其他异常 JSON 要失败，不能宽泛吞掉解析错误。

关键文件：`src/v14.rs`、`abi/v14/`、`proto/`、`substreams.yaml`、
`examples/make_v14_continuation.rs`、`scripts/audit-continuation-compatibility.sh`、
`scripts/migrate-rh-v14-schema.sql`、`scripts/check-rh-v14.sql`。
源地址和部署交易见 `deployments/4663/v14-sources.json`；语义覆盖表见 V14 compatibility 文档。

## 5. 包和回滚身份

| 身份 | 值 |
| --- | --- |
| 当前源码身份 | `7ff3065`，V0.6.1 |
| 当前 SPKG SHA-256 | `1af895c7b2f27f3bb978c1688f688ac1d1560fe104b200ef841b08cc4c8c3c14` |
| 当前输出 / hash | `v14_continuation_db_out` / `17a9486293f4cf6aaeb1b2e02724bae301332352` |
| 新域输出 / hash | `v14_backfill_db_out` / `ed554bb79c66f987eb110bc3fd54fc07c69b3d14` |
| 旧包 | `/opt/tiptag-substreams/tiptag-v11-cutover-v0.5.3.spkg` |
| 旧包 SHA-256 | `322550ec686f7c95e112b6462b365f2de4303cb8d2166c2b185d3dadd1747c6e` |
| 旧输出 / hash | `v11_continuation_db_out` / `40c74d5401d4caa30a8a38723d768b3ba3d627f1` |
| 旧系统表 | `unified_v52_cutover_cursors` / `unified_v52_cutover_substreams_history` |
| 切换 block ID | `fccc5d9d741e18b259ace6416781cba536a75769ed4753e07866deb064204f28` |

停稳的回滚备份：

- DB：`/root/backups/tiptag-rh-v14-cutover-78241605-20261008T113436Z.dump`
- SHA-256：`4ec3d7e974689a083355d7c726dce87b85c71bd3afca9e03980caa731e635816`
- 配置：`/root/backups/tiptag-rh-v14-cutover-78241605-20261008T113436Z.env`
- 本次窗口调整前配置：
  `/etc/tiptag-unified-substreams.env.before-window-300k.20261009T031241Z`

备份 TOC 可读已确认，真正恢复演练未执行。新统一索引已经提交了 C 后的旧业务增量，
回滚不能简单切回旧包和旧游标，否则可能重复累加。先按规范停写入，使用停稳备份恢复，
或另行证明并审查新的兼容边界；同时审查下游状态。

## 6. 性能排查结果

免费账号上限 5 个 worker 已由操作员确认。`5fba833` 增加可选
`SUBSTREAMS_PARALLEL_WORKERS` launcher 参数，但 **10-worker 试验已取消，未确认安装，
未配置为线上 10 worker**。以后不能把它当成本次提速措施。

30 万配置于北京时间 11:12:41 更新；下一轮北京时间 11:24:20 生效，
没有中断当前 run，没有改 SPKG、输出 hash 或数据库游标。

| 样本（服务器 +02:00） | 游标增量 | 耗时 | 已提交高度推进速度 |
| --- | ---: | ---: | ---: |
| 10 万基础窗口，01:04:50–04:51:08，含自适应窗口 | 623,930 | 3:46:18 | 约 46.0 块/秒 |
| 30 万，05:24:20–06:37:50 | 280,002 | 1:13:30 | 约 63.5 块/秒 |
| 30 万，06:37:51–08:17:54 | 297,212 | 1:40:03 | 约 49.5 块/秒 |

两轮 30 万合计约 55.4 块/秒，观测上比前一段高约 20%，但样本区间、事件和缓存不同，
不是受控性能证明。不要沿用只看第一轮得出的“稳定提升 38%”。
同期链头增长约 9.8 块/秒；ETA 应用积压除以**索引速度减链增长速度**，
且头高度和游标要取同一时间附近。对固定部署高度则不减链增长。
之前约 15 小时追头、7–8 小时到部署区间的估算是当时状态下的估算，不是当前承诺。

主要观察：连接初始化只花约 12–13 秒，轮次间隔很小；多数时间在远端执行或等待。
多层 store 依赖和 5-worker 并行上限值得关注。早先 PostgreSQL 约 30 MB、
事件写入稀疏，未见写入瓶颈证据。进一步精确区分远端计算、缓存读取、调度和合并
需要服务商侧信息，现有日志不能完成全部归因。

不要把 `progress_block_rate` 当作链高度推进速度，也不要把累计请求范围直接当收费量。
短期 rate=0、last_block=None 或 SQL 游标没变都不足以判定停滞；结合 stage 的前后变化、
任务、连接、错误、provider stream 和已提交游标。扩大窗口会减少一些重叠请求，
不增加 worker 数；已扫描但无新输出的区间下一轮可以再次请求，缓存复用情况需实测。

## 7. 下一次的具体起手步骤

以下是分别执行的只读步骤参考；实际协作时仍然一次只给一个命令并等待回传。

先读实际数据库游标，不使用本文件的历史高度直接启动任务：

```bash
docker exec tiptag-substreams-postgres sh -c 'psql -U "${POSTGRES_USER:-postgres}" -d tiptag_rh -X -w -v ON_ERROR_STOP=1 -P pager=off -c "SELECT id, block_num, block_id FROM public.unified_v14_cursors WHERE id = '\''17a9486293f4cf6aaeb1b2e02724bae301332352'\'';"'
```

再读轮次边界和最近两条远端进度，避免 `tail` 只保留统计而丢掉配置：

```bash
journalctl -u tiptag-unified-incremental.service \
  --since '6 hours ago' --no-pager -o short-iso |
  awk '
    /"event":"incremental_(target|window_state)"/ { print }
    /"message":"substreams stream stats"/ { previous=latest; latest=$0 }
    /"severity":"(ERROR|FATAL)"/ { print }
    END { if (previous) print previous; if (latest) print latest }
  '
```

然后分步重新确认 systemd 实际超时、非敏感配置、SPKG checksum、源码 Git 状态。
若新功能开始开发，先提取部署交易、ABI、最早源事件块和 BSC 语义差异；
重新选择 C/H 对应路径，再开发新域、审计、验证和安排迁移。

本次下一项验收：追赶经过 Router 和 Pump 部署区间后，由操作员运行
`scripts/check-rh-v14.sql`，验证初始 53 个价格池、105 条 route、52 个批准资产及事件。
数量描述部署初始化，后续管理操作可能改变当前快照；同时按具体交易核对。
创建、交易、上市、回购、认领等事件族还需各自实链和 SQL 验证。
随后处理 `tiptag-server` PostgreSQL → MySQL 投影、membership union view、
TagAI MySQL 迁移和 API 接口验收，最后验证 reorg/恢复流程并更新交接快照。
