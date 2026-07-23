#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const [endpoint, rawBlock, reportPath] = process.argv.slice(2);
if (!endpoint || !rawBlock || !/^\d+$/.test(rawBlock)) {
  console.error(
    "Usage: node scripts/compare-graph-postgres.mjs <graph-endpoint> <block> [report.json]",
  );
  process.exit(2);
}

const comparisonBlock = Number(rawBlock);
const pageSize = Number(process.env.GRAPH_PAGE_SIZE || 1000);
const sampleLimit = Number(process.env.MISMATCH_SAMPLE_LIMIT || 20);
const pgContainer = process.env.PG_CONTAINER || "tiptag-substreams-postgres";
const pgUser = process.env.PG_USER || "tiptag";
const pgDatabase = process.env.PG_DATABASE || "tiptag_rh";

const text = (value) => (value == null ? "" : String(value));
const numberText = (value) => String(value ?? 0);
const nullableNumberText = (value) =>
  value == null || value === "" ? null : String(value);
const lower = (value) => text(value).toLowerCase();
const relationId = (value) => lower(value?.id ?? value);
const hash = (value) => lower(value).replace(/^0x/, "");

function compact(row) {
  return { ...row };
}

function runPostgres(sql) {
  const result = spawnSync(
    "docker",
    [
      "exec",
      pgContainer,
      "psql",
      "-U",
      pgUser,
      "-d",
      pgDatabase,
      "-X",
      "-A",
      "-t",
      "-v",
      "ON_ERROR_STOP=1",
      "-c",
      sql,
    ],
    { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `PostgreSQL query failed:\n${result.stderr || result.stdout}`,
    );
  }
  const output = result.stdout.trim();
  if (!output) return [];
  return output.split("\n").map((line) => compact(JSON.parse(line)));
}

async function graphRequest(query, variables) {
  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });
  if (!response.ok) {
    throw new Error(`Graph HTTP ${response.status}: ${await response.text()}`);
  }
  const body = await response.json();
  if (body.errors) throw new Error(JSON.stringify(body.errors, null, 2));
  return body.data;
}

async function graphMeta() {
  const data = await graphRequest(
    `query AuditMeta($block: Int!) {
      _meta(block: { number: $block }) {
        block { number hash }
        deployment
        hasIndexingErrors
      }
    }`,
    { block: comparisonBlock },
  );
  return data._meta;
}

async function fetchGraphRows(spec) {
  const rows = [];
  let lastId = null;
  for (;;) {
    const variableDefinition = lastId
      ? "($block: Int!, $last: Bytes!)"
      : "($block: Int!)";
    const where = lastId ? ", where: { id_gt: $last }" : "";
    const query = `query Audit${variableDefinition} {
      rows: ${spec.root}(
        first: ${pageSize},
        orderBy: id,
        orderDirection: asc,
        block: { number: $block }${where}
      ) {
        ${spec.selection}
      }
    }`;
    const data = await graphRequest(query, {
      block: comparisonBlock,
      ...(lastId ? { last: lastId } : {}),
    });
    const page = data.rows;
    for (const row of page) rows.push(compact(spec.fromGraph(row)));
    if (page.length < pageSize) break;
    lastId = page[page.length - 1].id;
  }
  return rows;
}

function makeMap(rows, keyOf) {
  const map = new Map();
  const duplicates = [];
  for (const row of rows) {
    const key = String(keyOf(row));
    if (map.has(key)) {
      if (duplicates.length < sampleLimit) {
        duplicates.push({ key, first: map.get(key), duplicate: row });
      }
    } else {
      map.set(key, row);
    }
  }
  return { map, duplicates };
}

function compareRows(spec, graphRows, postgresRows) {
  const graph = makeMap(graphRows, spec.key);
  const postgres = makeMap(postgresRows, spec.key);
  const missingInPostgres = [];
  const extraInPostgres = [];
  const fieldMismatches = [];

  for (const [key, graphRow] of graph.map) {
    const postgresRow = postgres.map.get(key);
    if (!postgresRow) {
      if (missingInPostgres.length < sampleLimit) {
        missingInPostgres.push({ key, graph: graphRow });
      }
      continue;
    }
    const differingFields = {};
    for (const field of Object.keys(graphRow)) {
      if (graphRow[field] !== postgresRow[field]) {
        differingFields[field] = {
          graph: graphRow[field],
          postgres: postgresRow[field],
        };
      }
    }
    for (const field of Object.keys(postgresRow)) {
      if (!(field in graphRow)) {
        differingFields[field] = {
          graph: undefined,
          postgres: postgresRow[field],
        };
      }
    }
    if (
      Object.keys(differingFields).length > 0 &&
      fieldMismatches.length < sampleLimit
    ) {
      fieldMismatches.push({ key, fields: differingFields });
    }
  }

  for (const [key, postgresRow] of postgres.map) {
    if (!graph.map.has(key) && extraInPostgres.length < sampleLimit) {
      extraInPostgres.push({ key, postgres: postgresRow });
    }
  }

  let missingCount = 0;
  let extraCount = 0;
  let mismatchCount = 0;
  for (const key of graph.map.keys()) {
    if (!postgres.map.has(key)) missingCount += 1;
  }
  for (const key of postgres.map.keys()) {
    if (!graph.map.has(key)) extraCount += 1;
  }
  for (const [key, graphRow] of graph.map) {
    const postgresRow = postgres.map.get(key);
    if (!postgresRow) continue;
    const fields = new Set([
      ...Object.keys(graphRow),
      ...Object.keys(postgresRow),
    ]);
    if ([...fields].some((field) => graphRow[field] !== postgresRow[field])) {
      mismatchCount += 1;
    }
  }

  return {
    graphRows: graphRows.length,
    postgresRows: postgresRows.length,
    graphDuplicateKeys: graphRows.length - graph.map.size,
    postgresDuplicateKeys: postgresRows.length - postgres.map.size,
    missingInPostgres: missingCount,
    extraInPostgres: extraCount,
    fieldMismatches: mismatchCount,
    samples: {
      graphDuplicates: graph.duplicates,
      postgresDuplicates: postgres.duplicates,
      missingInPostgres,
      extraInPostgres,
      fieldMismatches,
    },
  };
}

const specs = [
  {
    name: "pump",
    root: "pumps",
    selection: "id tokenCounts listedCounts",
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: "pump",
      tokenCounts: numberText(row.tokenCounts),
      listedCounts: numberText(row.listedCounts),
    }),
    sql: `SELECT json_build_object(
        'id', 'pump',
      'tokenCounts', token_counts::text,
      'listedCounts', listed_counts::text
    ) FROM pump_summary ORDER BY id`,
  },
  {
    name: "tokens",
    root: "tokens",
    selection: `id index symbol listed creator { id } buyTimes sellTimes
      tiptagFee sellsmanFee price pump version bondingCurveSupply
      maxBondingCurveSupply`,
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: lower(row.id),
      index: numberText(row.index),
      symbol: text(row.symbol),
      listed: Boolean(row.listed),
      creator: relationId(row.creator),
      buyTimes: numberText(row.buyTimes),
      sellTimes: numberText(row.sellTimes),
      tiptagFee: numberText(row.tiptagFee),
      sellsmanFee: numberText(row.sellsmanFee),
      price: numberText(row.price),
      pump: lower(row.pump),
      version: numberText(row.version),
      bondingCurveSupply: numberText(row.bondingCurveSupply),
      maxBondingCurveSupply: numberText(row.maxBondingCurveSupply),
    }),
    sql: `SELECT json_build_object(
      'id', lower(id),
      'index', entity_index::text,
      'symbol', symbol,
      'listed', listed,
      'creator', lower(creator),
      'buyTimes', buy_times::text,
      'sellTimes', sell_times::text,
      'tiptagFee', tiptag_fee::text,
      'sellsmanFee', sellsman_fee::text,
      'price', price::text,
      'pump', lower(pump),
      'version', version::text,
      'bondingCurveSupply', bonding_curve_supply::text,
      'maxBondingCurveSupply', max_bonding_curve_supply::text
    ) FROM tokens ORDER BY id`,
  },
  {
    name: "tokenTrades",
    root: "tokenTrades",
    selection: `id index trader { id } token { id } isBuy amount ethAmount
      timestamp sellsman { id } sellsmanFee tiptagFee price transHash`,
    key: (row) => row.index,
    fromGraph: (row) => ({
      index: numberText(row.index),
      trader: relationId(row.trader),
      token: relationId(row.token),
      isBuy: Boolean(row.isBuy),
      amount: numberText(row.amount),
      ethAmount: numberText(row.ethAmount),
      timestamp: numberText(row.timestamp),
      sellsman: relationId(row.sellsman),
      sellsmanFee: numberText(row.sellsmanFee),
      tiptagFee: numberText(row.tiptagFee),
      price: numberText(row.price),
      transHash: hash(row.transHash),
    }),
    sql: `SELECT json_build_object(
      'index', entity_index::text,
      'trader', lower(buyer),
      'token', lower(token),
      'isBuy', is_buy,
      'amount', token_amount::text,
      'ethAmount', eth_amount::text,
      'timestamp', block_timestamp::text,
      'sellsman', lower(sellsman),
      'sellsmanFee', sellsman_fee::text,
      'tiptagFee', tiptag_fee::text,
      'price', price::text,
      'transHash', lower(regexp_replace(transaction_hash, '^0x', ''))
    ) FROM token_trade_events ORDER BY entity_index`,
  },
  {
    name: "listedTokens",
    root: "listedTokens",
    selection: "id index token { id } blockNum timestamp pair",
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: lower(row.id),
      index: numberText(row.index),
      token: relationId(row.token),
      blockNum: numberText(row.blockNum),
      timestamp: numberText(row.timestamp),
      pair: lower(row.pair),
    }),
    sql: `SELECT json_build_object(
      'id', lower(token),
      'index', entity_index::text,
      'token', lower(token),
      'blockNum', block_number::text,
      'timestamp', block_timestamp::text,
      'pair', lower(pool_id)
    ) FROM token_listings ORDER BY token`,
  },
  {
    name: "pairs",
    root: "pairs",
    selection: "id token { id } tokenIndex",
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: lower(row.id),
      token: relationId(row.token),
      tokenIndex: numberText(row.tokenIndex),
    }),
    sql: `SELECT json_build_object(
      'id', lower(id),
      'token', lower(token),
      'tokenIndex', token_index::text
    ) FROM pairs ORDER BY id`,
  },
  {
    name: "ipshareSummary",
    root: "ipshareSummaries",
    selection: `id usersCount totalProtocolFee totalCreateFee buyCount
      sellCount totalValueCapture`,
    key: () => "summary",
    fromGraph: (row) => ({
      id: "summary",
      usersCount: numberText(row.usersCount),
      totalProtocolFee: numberText(row.totalProtocolFee),
      totalCreateFee: numberText(row.totalCreateFee),
      buyCount: numberText(row.buyCount),
      sellCount: numberText(row.sellCount),
      totalValueCapture: numberText(row.totalValueCapture),
    }),
    sql: `SELECT json_build_object(
      'id', 'summary',
      'usersCount', users_count::text,
      'totalProtocolFee', total_protocol_fee::text,
      'totalCreateFee', total_create_fee::text,
      'buyCount', buy_count::text,
      'sellCount', sell_count::text,
      'totalValueCapture', total_value_capture::text
    ) FROM ipshare_summary`,
  },
  {
    name: "ipshareTrades",
    root: "trades",
    selection: `id index trader { id } subject { id } isBuy shareAmount
      ethAmount protocolEthAmount subjectEthAmount supply timestamp`,
    key: (row) => row.index,
    fromGraph: (row) => ({
      index: numberText(row.index),
      trader: relationId(row.trader),
      subject: relationId(row.subject),
      isBuy: Boolean(row.isBuy),
      shareAmount: numberText(row.shareAmount),
      ethAmount: numberText(row.ethAmount),
      protocolEthAmount: numberText(row.protocolEthAmount),
      subjectEthAmount: numberText(row.subjectEthAmount),
      supply: numberText(row.supply),
      timestamp: numberText(row.timestamp),
    }),
    sql: `SELECT json_build_object(
      'index', entity_index::text,
      'trader', lower(trader),
      'subject', lower(subject),
      'isBuy', is_buy,
      'shareAmount', share_amount::text,
      'ethAmount', eth_amount::text,
      'protocolEthAmount', protocol_eth_amount::text,
      'subjectEthAmount', subject_eth_amount::text,
      'supply', supply::text,
      'timestamp', block_timestamp::text
    ) FROM ipshare_trade_events ORDER BY entity_index`,
  },
  {
    name: "valueCaptured",
    root: "valueCaptureds",
    selection: "id index subject { id } investor { id } amount timestamp",
    key: (row) => row.index,
    fromGraph: (row) => ({
      index: numberText(row.index),
      subject: relationId(row.subject),
      investor: relationId(row.investor),
      amount: numberText(row.amount),
      timestamp: numberText(row.timestamp),
    }),
    sql: `SELECT json_build_object(
      'index', entity_index::text,
      'subject', lower(subject),
      'investor', lower(investor),
      'amount', amount::text,
      'timestamp', block_timestamp::text
    ) FROM ipshare_value_capture_events ORDER BY entity_index`,
  },
  {
    name: "stakes",
    root: "stakes",
    selection: `id index staker { id } subject { id } isStake shareAmount time`,
    key: (row) => row.index,
    fromGraph: (row) => ({
      index: numberText(row.index),
      staker: relationId(row.staker),
      subject: relationId(row.subject),
      isStake: Boolean(row.isStake),
      shareAmount: numberText(row.shareAmount),
      time: numberText(row.time),
    }),
    sql: `SELECT json_build_object(
      'index', entity_index::text,
      'staker', lower(staker),
      'subject', lower(subject),
      'isStake', is_stake,
      'shareAmount', share_amount::text,
      'time', block_timestamp::text
    ) FROM ipshare_stake_events ORDER BY entity_index`,
  },
  {
    name: "holders",
    root: "holders",
    selection: "id createAt holder { id } subject { id } sharesOwned",
    key: (row) => `${row.holder}:${row.subject}`,
    fromGraph: (row) => ({
      holder: relationId(row.holder),
      subject: relationId(row.subject),
      createAt: numberText(row.createAt),
      sharesOwned: numberText(row.sharesOwned),
    }),
    sql: `SELECT json_build_object(
      'holder', lower(holder),
      'subject', lower(subject),
      'createAt', COALESCE(created_at, 0)::text,
      'sharesOwned', shares_owned::text
    ) FROM ipshare_holders ORDER BY holder, subject`,
  },
  {
    name: "stakers",
    root: "stakers",
    selection: "id createAt staker { id } subject { id } stakedAmount",
    key: (row) => `${row.staker}:${row.subject}`,
    fromGraph: (row) => ({
      staker: relationId(row.staker),
      subject: relationId(row.subject),
      createAt: numberText(row.createAt),
      stakedAmount: numberText(row.stakedAmount),
    }),
    sql: `SELECT json_build_object(
      'staker', lower(staker),
      'subject', lower(subject),
      'createAt', COALESCE(created_at, 0)::text,
      'stakedAmount', staked_amount::text
    ) FROM ipshare_stakers ORDER BY staker, subject`,
  },
  {
    name: "accounts",
    root: "accounts",
    selection: `id joinIn index ipShareIndex holdersCount holdingsCount
      shareSupply ipshareCreateBlock stakersCount stakedCount feeAmount
      captureCount totalCaptured totalStaked walnutOperationCount`,
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: lower(row.id),
      joinIn: numberText(row.joinIn),
      index: numberText(row.index),
      ipShareIndex: numberText(row.ipShareIndex),
      holdersCount: numberText(row.holdersCount),
      holdingsCount: numberText(row.holdingsCount),
      shareSupply: numberText(row.shareSupply),
      ipshareCreateBlock: numberText(row.ipshareCreateBlock),
      stakersCount: numberText(row.stakersCount),
      stakedCount: numberText(row.stakedCount),
      feeAmount: numberText(row.feeAmount),
      captureCount: numberText(row.captureCount),
      totalCaptured: numberText(row.totalCaptured),
      totalStaked: numberText(row.totalStaked),
      walnutOperationCount: numberText(row.walnutOperationCount),
    }),
    sql: `SELECT json_build_object(
      'id', lower(id),
      'joinIn', COALESCE(joined_at, 0)::text,
      'index', entity_index::text,
      'ipShareIndex', ipshare_index::text,
      'holdersCount', holders_count::text,
      'holdingsCount', holdings_count::text,
      'shareSupply', share_supply::text,
      'ipshareCreateBlock', ipshare_create_block::text,
      'stakersCount', stakers_count::text,
      'stakedCount', staked_count::text,
      'feeAmount', fee_amount::text,
      'captureCount', capture_count::text,
      'totalCaptured', total_captured::text,
      'totalStaked', total_staked::text,
      'walnutOperationCount', walnut_operation_count::text
    ) FROM accounts ORDER BY id`,
  },
  {
    name: "walnutSummary",
    root: "walnuts",
    selection: "id tvl totalCommunities totalUsers totalPools",
    key: () => "walnut",
    fromGraph: (row) => ({
      id: "walnut",
      tvl: numberText(row.tvl),
      totalCommunities: numberText(row.totalCommunities),
      totalUsers: numberText(row.totalUsers),
      totalPools: numberText(row.totalPools),
    }),
    sql: `SELECT json_build_object(
      'id', 'walnut',
      'tvl', tvl::text,
      'totalCommunities', total_communities::text,
      'totalUsers', total_users::text,
      'totalPools', total_pools::text
    ) FROM walnut_summary`,
  },
  {
    name: "communities",
    root: "communities",
    selection: `id index createdAt owner { id } daoFund feeRatio cToken
      treasury distributedCToken revenue retainedRevenue usersCount poolsCount
      activePoolCount operationCount`,
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: lower(row.id),
      index: numberText(row.index),
      createdAt: numberText(row.createdAt),
      owner: relationId(row.owner),
      daoFund: lower(row.daoFund),
      feeRatio: numberText(row.feeRatio),
      cToken: lower(row.cToken),
      treasury: lower(row.treasury),
      distributedCToken: numberText(row.distributedCToken),
      revenue: numberText(row.revenue),
      retainedRevenue: numberText(row.retainedRevenue),
      usersCount: numberText(row.usersCount),
      poolsCount: numberText(row.poolsCount),
      activePoolCount: numberText(row.activePoolCount),
      operationCount: numberText(row.operationCount),
    }),
    sql: `SELECT json_build_object(
      'id', lower(id),
      'index', entity_index::text,
      'createdAt', created_at::text,
      'owner', lower(owner),
      'daoFund', lower(dao_fund),
      'feeRatio', fee_ratio::text,
      'cToken', lower(c_token),
      'treasury', lower(treasury),
      'distributedCToken', distributed_c_token::text,
      'revenue', revenue::text,
      'retainedRevenue', retained_revenue::text,
      'usersCount', users_count::text,
      'poolsCount', pools_count::text,
      'activePoolCount', active_pool_count::text,
      'operationCount', operation_count::text
    ) FROM walnut_communities ORDER BY id`,
  },
  {
    name: "pools",
    root: "pools",
    selection: `id index poolIndex createdAt status name poolFactory
      community { id } ratio asset chainId totalAmount tvl stakersCount
      lockDuration poolType`,
    key: (row) => row.id,
    fromGraph: (row) => ({
      id: lower(row.id),
      index: numberText(row.index),
      poolIndex: numberText(row.poolIndex),
      createdAt: numberText(row.createdAt),
      status: text(row.status),
      name: text(row.name),
      poolFactory: lower(row.poolFactory),
      community: relationId(row.community),
      ratio: numberText(row.ratio),
      asset: lower(row.asset),
      chainId: nullableNumberText(row.chainId),
      totalAmount: numberText(row.totalAmount),
      tvl: nullableNumberText(row.tvl),
      stakersCount: numberText(row.stakersCount),
      lockDuration: nullableNumberText(row.lockDuration),
      poolType: text(row.poolType),
    }),
    sql: `SELECT json_build_object(
      'id', lower(id),
      'index', entity_index::text,
      'poolIndex', pool_index::text,
      'createdAt', created_at::text,
      'status', status,
      'name', name,
      'poolFactory', lower(pool_factory),
      'community', lower(community),
      'ratio', ratio::text,
      'asset', lower(asset),
      'chainId', CASE WHEN chain_id IS NULL THEN NULL ELSE chain_id::text END,
      'totalAmount', total_amount::text,
      'tvl', CASE WHEN tvl IS NULL THEN NULL ELSE tvl::text END,
      'stakersCount', stakers_count::text,
      'lockDuration', CASE WHEN lock_duration IS NULL THEN NULL ELSE lock_duration::text END,
      'poolType', pool_type
    ) FROM walnut_pools ORDER BY id`,
  },
  {
    name: "walnutOperations",
    root: "userOperationHistories",
    selection: `id index type community { id } poolFactory pool { id }
      account { id } chainId asset amount timestamp tx socialOrderId
      socialHarvested`,
    key: (row) => row.index,
    fromGraph: (row) => ({
      index: numberText(row.index),
      type: text(row.type),
      community: relationId(row.community),
      poolFactory: row.poolFactory == null ? null : lower(row.poolFactory),
      pool: row.pool == null ? null : relationId(row.pool),
      account: relationId(row.account),
      chainId: nullableNumberText(row.chainId),
      asset: row.asset == null ? null : lower(row.asset),
      amount: nullableNumberText(row.amount),
      timestamp: numberText(row.timestamp),
      tx: hash(row.tx),
      socialOrderId: nullableNumberText(row.socialOrderId),
      socialHarvested:
        row.socialHarvested == null ? null : Boolean(row.socialHarvested),
    }),
    sql: `SELECT json_build_object(
      'index', entity_index::text,
      'type', operation_type,
      'community', lower(community),
      'poolFactory', CASE WHEN pool_factory IS NULL THEN NULL ELSE lower(pool_factory) END,
      'pool', CASE WHEN pool IS NULL THEN NULL ELSE lower(pool) END,
      'account', lower(account),
      'chainId', CASE WHEN chain_id IS NULL THEN NULL ELSE chain_id::text END,
      'asset', CASE WHEN asset IS NULL THEN NULL ELSE lower(asset) END,
      'amount', CASE WHEN amount IS NULL THEN NULL ELSE amount::text END,
      'timestamp', block_timestamp::text,
      'tx', lower(regexp_replace(transaction_hash, '^0x', '')),
      'socialOrderId', CASE WHEN social_order_id IS NULL THEN NULL ELSE social_order_id::text END,
      'socialHarvested', social_harvested
    ) FROM walnut_operations ORDER BY entity_index`,
  },
];

const startedAt = new Date().toISOString();
const meta = await graphMeta();
if (meta.hasIndexingErrors) {
  throw new Error("Graph reports hasIndexingErrors=true");
}
if (Number(meta.block.number) !== comparisonBlock) {
  throw new Error(
    `Graph returned block ${meta.block.number}, expected ${comparisonBlock}`,
  );
}

const report = {
  startedAt,
  completedAt: null,
  comparisonBlock,
  graphMeta: meta,
  exclusions: [
    "TokenTransfer: intentionally not indexed by Substreams",
    "TokenHolder/token balances/holdersCount/age: refreshed from Blockscout",
    "Basket entities: outside this TagAI/Nutbox audit",
  ],
  entities: {},
  totals: {
    missingInPostgres: 0,
    extraInPostgres: 0,
    fieldMismatches: 0,
    duplicateSemanticKeys: 0,
  },
};

for (const spec of specs) {
  process.stderr.write(`Comparing ${spec.name}... `);
  const [graphRows, postgresRows] = await Promise.all([
    fetchGraphRows(spec),
    Promise.resolve(runPostgres(spec.sql)),
  ]);
  const result = compareRows(spec, graphRows, postgresRows);
  report.entities[spec.name] = result;
  report.totals.missingInPostgres += result.missingInPostgres;
  report.totals.extraInPostgres += result.extraInPostgres;
  report.totals.fieldMismatches += result.fieldMismatches;
  report.totals.duplicateSemanticKeys +=
    result.graphDuplicateKeys + result.postgresDuplicateKeys;
  process.stderr.write(
    `graph=${result.graphRows} postgres=${result.postgresRows} ` +
      `missing=${result.missingInPostgres} extra=${result.extraInPostgres} ` +
      `mismatch=${result.fieldMismatches}\n`,
  );
}

report.completedAt = new Date().toISOString();
const rendered = `${JSON.stringify(report, null, 2)}\n`;
if (reportPath) writeFileSync(reportPath, rendered);
process.stdout.write(rendered);

if (
  report.totals.missingInPostgres > 0 ||
  report.totals.extraInPostgres > 0 ||
  report.totals.fieldMismatches > 0 ||
  report.totals.duplicateSemanticKeys > 0
) {
  process.exitCode = 1;
}
