# RH ImportHelperV2 索引与发布记录

2026-10-12：本地实现已完成；操作员已通过服务器构建、精确旧包兼容审计和两个部署块的 Pinax 只读回放。
PostgreSQL 迁移及停稳备份已完成，独立域历史补索引已正式启动；范围完成、统一索引切换、续跑和产品验收仍待操作员逐步执行。
本文件接续 `SUBSTREAMS_AGENT_RUNBOOK.md`、`RH_V11_COMPATIBILITY.md` 和 `RH_V14_HANDOFF.md`。
开发期间保持生产索引运行；服务器一次给一个命令，收到真实结果后再给下一步。

## 已确认线上身份

操作员本次回传：

- 数据库 `tiptag_rh`，容器 `tiptag-substreams-postgres`。
- `/opt/tiptag-substreams/tiptag-v14-continuation-v0.6.1.spkg`。
- SPKG SHA-256 `1af895c7b2f27f3bb978c1688f688ac1d1560fe104b200ef841b08cc4c8c3c14`。
- 输出 `v14_continuation_db_out`，hash `17a9486293f4cf6aaeb1b2e02724bae301332352`。
- `unified_v14_cursors` / `unified_v14_substreams_history`。
- 已提交块 **85,970,291**，block ID `6252c4a1dacfed0a2dc0a065665a66d288d41227e4384ba0e6877380ef797091`。

这不是停稳的最终切换边界。切换时必须重新读取 C；不能把当前高度用于制造或复制游标。
新 Helper 部署块 86,448,655 尚在当前已提交游标前方；但当前 Wrapper 从 54,128,714 开始，
现有旧模块只覆盖另一个历史 Wrapper 地址，所以新增域需要独立补齐该地址的历史。

## 合约来源

`deployments/4663/import-v2-sources.json` 记录独立 RPC 核验的成功部署收据和 canonical block。
公开日志样本在 `fixtures/rh-import-v2-deployment-logs.json`，事件 ABI 在 `abi/import-v2/`。

| 合约 | 地址 | 起点 |
| --- | --- | ---: |
| 当前 Wrapper | `0x53e65de68a0eb7f3662579f44eb9849ae5ca44ab` | 54,128,714 |
| 旧 RH Helper | `0xd0baaf8d314b0c6ab6bebd24ddbc86157c9c7b09` | 54,128,745 |
| 新 ImportHelperV2 | `0x70ed7c79d80dede1828a09b3c526df283111cb82` | 86,448,655 |

历史 `0xddf749…` Helper / `0x91ddca…` Wrapper 的既有模块继续由原生产包负责。
更早 `0xEC774…` Helper 的 API 注册恢复白名单保留；本次新索引域限定上述三个地址。
不宣称修复其它未列入来源的历史地址。

## 独立模块与语义

模板版本 `v0.7.0`，新增三个模块：

- `map_import_v2_events`：仅三个精确 `evt_addr`，分别验证部署边界；完整 receipt、全局 log index，
  成功交易、重复日志去重、ABI 命名字段和 ordinal；已知事件损坏时失败。
- `import_v2_backfill_db_out`：只写 `import_v2_events`、`import_v2_markets`、`import_v2_trade_events`。
  不依赖任何历史 store，不改旧 Token/Walnut/V14 聚合。
- `import_v2_continuation_db_out`：实际旧 sink + 新域输出的 ordinal 合并。

Wrapper 的 `ImportedTokenTrade` 是成交数量依据，覆盖 V2/V3/Uniswap V4/Pancake V4 的买卖、
实际 tokenAmount、grossNativeAmount、nutboxTokenFee、recipient 和 sellsman。
普通 `Trade` 只归档，不重复创建成交行；不添加 Swap/Transfer 公共签名或持仓索引。
`call_index` 在此独立域表示全局日志位置，ID 为 tx hash + log index，索引为 block*1,000,000+log。
所有写入为稳定主键的 Set/Upsert，无 Add。部分市场更新的缺失字段用 schema 默认值引导。
V2 首个矿池可能是 TradeCuration，不能将它写成 social_pool；Social 链接按实际工厂确认。
矿池及 Community 的 canonical 状态继续由已保留的 Walnut/V14 工厂模块负责。

## 组装和部署门槛

在服务器从**精确已安装包**组装，不能直接运行开发模板：

```bash
cargo run --release --example make_import_v2_continuation -- \
  /opt/tiptag-substreams/tiptag-v14-continuation-v0.6.1.spkg \
  tiptag-substreams-v0.7.0.spkg tiptag-import-v2-continuation-v0.7.0.spkg
```

用 `audit-continuation-compatibility.sh` 比较实际旧输出与 `import_v2_continuation_db_out`，
预期旧 **74 个 shared modules 全部不变**，新增 3 个。组装器复制旧 definitions、WASM、
filters 和 network overrides，并继承实际旧输出起点。缺少旧模块或新名称碰撞时拒绝组装。
派生 `import_v2_backfill_db_out` 包；分别记录两个包 checksum 和输出 hash。
本地模板/本地历史模板组装结果仅用于结构验证，不是生产发布身份。

### 本次操作员确认的服务器候选包

实际身份及已通过门槛记录在 `deployments/4663/import-v2-server-candidate.json`：

| 输出 | SPKG SHA-256 | 模块 hash | 起点 |
| --- | --- | --- | ---: |
| continuation | `7dba55e72ca23cc524d8085636a94a087c5f6f632c6e04b1c257d221f537b125` | `d971756177525a8da560588154366400a9f48164` | 53,869,281 |
| backfill | `a3d04f579b08a338d51667eff22e4ba0f05206a689b7a16f20fbf40bda7724f2` | `aacb951d3546d75d6aa6da6abe764feb48f56cee` | 54,128,714 |

服务器 47 个 library tests、2 个 assembler tests 通过，旧模块审计为 74/74 unchanged、0 changed。
新 Helper 部署块 86,448,655 的回放报告处理/接收各 1 块；输出 4 行独立事件，日志位置 40–43，
默认 Trade Curation 工厂比例 10000。Wrapper 部署块 54,128,714 输出 1 行 OwnershipTransferred，
日志位置 63。两组事件的地址、交易和区块 hash 均与已核验公开收据一致。
只读输出均仅涉及 `import_v2_events`；这不是 PostgreSQL 入库或真实交易业务验收。
服务器 Git 完整 commit 和工具版本仍待记录；当前生产包和游标尚未切换。

迁移前备份（生产索引仍运行，并非最终停稳切换备份）：
`/root/backups/tiptag-rh-before-import-v2-20261012T084934Z.dump`，操作员报告大小 2.3M，
SHA-256 `4a93174d0ea3b15e2586556d0ea54bcafe6a6b780b17329aa103b1601f9208cb`。
`pg_restore --list` 读取成功，35K 目录保存在同路径 `.toc` 文件；尚未执行恢复演练。
操作员随后在 `tiptag_rh` / `public` 执行迁移，回传 3 次 CREATE TABLE、2 次 CREATE INDEX
和 COMMIT。随后只读检查确认 3 张表均有主键，事件/市场/成交计数均为 0，
旧游标仍为 85,970,291，block ID 与前次一致。补索引与统一包切换尚未完成，
该运行中游标仍不能作为最终停稳边界。
操作员已用 backfill 包运行 `setup --system-tables-only`，专用系统表为
`import_v2_backfill_cursors` / `import_v2_backfill_substreams_history`，hash policy 为 error；
回传 setup completed successfully。此步骤仅准备系统表，历史补索引尚未启动。
统一包的 `setup --system-tables-only` 也已回传成功，系统表为
`unified_import_v2_cursors` / `unified_import_v2_substreams_history`，hash policy 同样为 error。
该准备步骤完成时仍未启动新 sink，也未停止旧索引；操作员负责实际启停服务。

随后用户要求优先完成索引，暂不调整应用侧服务。给出的停止命令按顺序停止旧 timer/service，
包含 MainPID=0 的检查，然后读取旧游标。操作员回传边界 C=85,970,291，
block ID `6252c4a1dacfed0a2dc0a065665a66d288d41227e4384ba0e6877380ef797091`，
新统一输出起点 C+1=85,970,292。回传未包含 systemd 状态段，停稳备份步骤再次检查停机状态。
先完成独立域补索引和统一索引；应用侧部署与新来源身份配置仍留在索引验收之后，
不得提前让下游切到尚未就绪的 `unified_import_v2_cursors`。

停稳备份已通过操作员检查（命令先检查 service MainPID=0 和 timer inactive）：
`/root/backups/tiptag-rh-import-v2-cutover-85970291-20261012T085902Z.dump`，2.3M，
SHA-256 `44bbffa80bc110a928256e19471af4fa2774190540a2d388018655c3a76de416`。
同前缀 `.dump.toc` 可读；`.env` / `.substreams.env` 为受限权限的配置备份，不记录秘密内容。
独立域补索引目标为 `[54,128,714, 85,970,292)`，新 Helper 部署块在该范围之后，
其部署及业务事件由切换后的统一输出捕获。旧索引保持停止，直到统一索引 canary 完成。

### 正式补索引已启动，范围尚未完成

操作员回传 2026-10-12 11:01:34–11:01:51 +02:00 日志：已发生第一次 PostgreSQL flush，
最后已写入块 54,128,714；stream 最后输出块 54,128,775；远端 stage 0 从 54,146,000
推进到 54,160,000，运行任务数达到 5，实际 processed blocks 为 2。
运行日志保存为 `/root/backups/rh-import-v2-backfill-to-85970291.log`。
以上是操作员回传时的快照，不能当作实时状态或范围完成证明。stage 进度、输出事件块和
数据库提交块分别记录；无事件范围不一定推进 SQL 游标，须最终成功结束及入库验收后再切统一包。

前台日志持续输出后，操作员按 Ctrl+C 中断。随后只读检查回传旧 timer inactive/dead、
旧 service failed 且 MainPID=0、ImportBackfillPID=none。没有同时运行的 sink。
不执行曾提供的 nohup 启动方案；改用独立 `tiptag-import-v2-backfill.service` 后台续跑，
输出进入 journal，范围与专用系统表保持不变，严格 hash 按已有游标恢复。
该临时服务只负责有结束区块的历史补索引，正式统一索引仍使用既有
`tiptag-unified-incremental.timer` / `.service`，并未替换生产调度方式。

操作员随后创建临时 systemd oneshot 服务（`systemd-run --no-block`，TimeoutStartSec=12h，
TimeoutStopSec=90s，stdout/stderr 进入 journal），并回传实际启动日志：PID 818489，
activating/start；Production 模式，77 modules，输出 hash
`aacb951d3546d75d6aa6da6abe764feb48f56cee`，stop exclusive=85,970,292。
数据库恢复块为 54,360,564，block ID
`f19c8fea7969ea4747dc81b22a87c9cb077cc2908c549676b228a90f26f29292`；
2026-10-12 11:10:31 +02:00 的远端会话确认 resolved_start_block=54,360,565、5 worker。
中断后按原游标续跑的门槛已通过，但整个补索引范围尚未完成，不能提前切统一包。
上述 PID/高度为回传时快照；临时单位完成后可能被 systemd 回收，完成验收需保留 journal。

发布顺序（实际操作仍拆成一个命令一轮）：

1. 核对服务器 Git 状态；拉取代码，测试、构建并从精确旧包组装、审计。
2. 对已知 Wrapper / 新 Helper 部署块执行 store-free 有界 Pinax 探针，核对公开日志。
   真实导入、买卖和费用注入样本需各自验收；部署日志不是这些业务的实链证明。
3. PostgreSQL 备份、TOC/hash 验证后，由操作员运行 `scripts/migrate-rh-import-v2-schema.sql`。
   新增 3 张有主键的独立表，无 DROP/TRUNCATE，也不改变旧业务表。
4. 到审查过的切换点，暂停下游 RH graph-data-sync / k-chart / reward worker，停旧 timer/service，
   记录最终 C/block ID，备份停稳的 DB、配置与 MySQL。
5. 用独立系统表 `import_v2_backfill_cursors` / `import_v2_backfill_substreams_history`，
   严格 hash，在 `[54,128,714, C+1)` 仅补新域。免费版仍最多 5 worker；若不能同时开 stream，
   在该已记录的切换点顺序补齐。成功到达 exclusive stop 与稀疏 processed-block ratio 都需证据，
   不能把稀疏 SQL 最后事件块当作整个范围完成。
6. 检查事件、市场、成交、重复处理、停止重启后的原游标续跑，以及 legacy 表指纹未变。
7. 建 `unified_import_v2_cursors` / `unified_import_v2_substreams_history`；安装已审计的统一包，
   `START_BLOCK=C+1`、`INCREMENTAL_CURSOR_ID=<新输出实际hash>`、hash policy=error。
   不复制 opaque cursor，不合并两个输出游标；先 1,000 块 canary，再恢复既有自适应窗口。
8. 验证新统一输出非零 checkpoint、停止重启与严格 hash 接续，然后迁移 MySQL / 部署 server。
   已经补到 C 的新域不会再次输出 C 之前的 SQL；旧模块 cache 身份继续保持。
9. 更新 API，再部署 UI；验收无 IPShare 导入、注册重试、交易列表、意图发现、奖励与 claim。

不要盲目扩大 limit-processed-blocks。新 backfill 只有精确地址、无 store；新统一输出仍含旧图，
provider cache/preparation 需要实际验证。

## 下游接入

`tiptag-server` 的 import 域有独立市场/成交游标，复用既有 MySQL 市场及成交表，
并将已注册社区的成交写入 RH trade_list。ERC20 decimals 使用 checkpoint 固定块的 Multicall3。
旧市场快照不可覆盖 updated_block 更大的新状态；写入失败不得推进 import 游标。
奖励发现分别保留 core after 与 importAfter，新域补历史不会被已有 core 游标跳过。

MySQL 迁移为 server 的 `v47-rh-import-helper-v2-raw-amounts.sql`：三个原始金额改 varchar(80)，
来源改 varchar(42)。只改字段宽度，保留现有行/主键/游标；完整 SQL 由操作员直接执行。

server 的 `SUBSTREAMS_ACTIVE_MODULE_HASH` / `SUBSTREAMS_ACTIVE_CURSOR_TABLE` 组成精确来源身份，
缺省兼容当前 V0.6.1；切换后必须使用新统一包实测 hash 和 unified_import_v2_cursors。
两个设置必须同时给出；非法 hash/table 拒绝启动。graph-data-sync 与 RH trade-curation 同步配置。
不增加产品开关；新表/迁移未就绪时明确失败，不能假称导入支持已经上线。

## 本地验证与待完成项

已通过 Rust 47 library tests、2 assembler boundary tests；Substreams build/info；
本地旧模板对比 74/74 unchanged、0 changed。公开 RPC 核验 3 笔部署收据，真实部署日志解码通过。
API 验证、新旧 Helper、单池/混合池与注册入库测试，UI MC wire tests、类型与生产构建，
server 奖励/投影/独立游标测试，以及临时 MySQL 迁移重复执行、uint256、幂等、旧状态防回退、
失败事务回滚均已通过。详细计数与身份写入 local-validation JSON。

仍须操作员完成：真实导入/交易事件的 Pinax 验收、PostgreSQL 迁移/入库和恢复验证、
MySQL v47、生产服务部署，以及真正的 RH V2 导入/买卖/奖励认领闭环。
本地没有 PostgreSQL 服务，不能把单元测试描述成 PostgreSQL 集成验收。

## 回滚

保留切换前 dump/env、旧 SPKG、旧/新系统表和最终 C。新统一输出会提交 C 之后的旧业务增量；
不能只切回旧 C 游标重跑，否则旧 Add 聚合可能重复。按 runbook 先停写入，使用停稳备份恢复，
或另行审查一个无重叠兼容边界；同步审查 MySQL 新域投影。既有合约 registrar 已切到新 Helper，
回滚旧 API/UI 还需核对这一链上事实，单独切旧应用可能阻断导入。Wrapper 的 Safe ownership
acceptance 与本次索引发布独立记录，未完成前不能声称已移交。
