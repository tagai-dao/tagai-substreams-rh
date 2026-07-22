#!/usr/bin/env node

const endpoint = process.argv[2];

if (!endpoint) {
  console.error("Usage: node scripts/audit-graph-holders.mjs <graph-endpoint>");
  process.exit(2);
}

const pageSize = 1000;
let lastId = null;
let total = 0;
let block = null;
const pairToIds = new Map();
const nonCanonical = [];
let nonCanonicalCount = 0;

async function queryPage(after) {
  const where = after ? ", where: { id_gt: $after }" : "";
  const variableDefinition = after ? "($after: Bytes!)" : "";
  const query = `query ${variableDefinition} {
    holders(first: ${pageSize}, orderBy: id, orderDirection: asc${where}) {
      id
      holder { id }
      subject { id }
    }
    _meta { block { number } deployment hasIndexingErrors }
  }`;

  const response = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ query, variables: after ? { after } : {} }),
  });
  if (!response.ok) throw new Error(`Graph HTTP ${response.status}`);

  const body = await response.json();
  if (body.errors) throw new Error(JSON.stringify(body.errors));
  return body.data;
}

for (;;) {
  const data = await queryPage(lastId);
  block = data._meta.block.number;

  for (const row of data.holders) {
    const holder = row.holder.id.toLowerCase();
    const subject = row.subject.id.toLowerCase();
    const id = row.id.toLowerCase();
    const pair = `${holder}:${subject}`;
    const expectedId = `0x${holder.slice(2)}${subject.slice(2)}`;

    const ids = pairToIds.get(pair) ?? [];
    ids.push(id);
    pairToIds.set(pair, ids);

    if (id !== expectedId) {
      nonCanonicalCount += 1;
      if (nonCanonical.length < 20) {
        nonCanonical.push({ id, expectedId, holder, subject });
      }
    }
  }

  total += data.holders.length;
  if (data.holders.length < pageSize) break;
  lastId = data.holders[data.holders.length - 1].id;
}

const duplicatePairs = [];
for (const [pair, ids] of pairToIds) {
  if (new Set(ids).size > 1) duplicatePairs.push({ pair, ids });
}

console.log(JSON.stringify({
  block,
  holders: total,
  semanticPairs: pairToIds.size,
  duplicatePairCount: duplicatePairs.length,
  nonCanonicalIdCount: nonCanonicalCount,
  duplicatePairSamples: duplicatePairs.slice(0, 20),
  nonCanonicalIdSamples: nonCanonical,
}, null, 2));

if (duplicatePairs.length > 0 || nonCanonicalCount > 0) process.exitCode = 1;
