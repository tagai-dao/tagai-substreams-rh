#![allow(deprecated)] // Graph EntityChanges v1 retains legacy ordinal/old_value fields.

mod abi;
#[allow(unused)]
mod pb;
use base64::encode as base64_encode;
use hex_literal::hex;
use pb::contract::v1 as contract;
use pb::sf::substreams::sink::entity::v1::{
    entity_change::Operation, value::Typed, EntityChange, EntityChanges, Field, Value,
};
use substreams::scalar::BigInt;
use substreams::store::{
    DeltaBigInt, DeltaInt64, DeltaString, Deltas, StoreAdd, StoreAddBigInt, StoreAddInt64,
    StoreGet, StoreGetBigInt, StoreGetInt64, StoreGetString, StoreNew, StoreSet,
    StoreSetIfNotExists, StoreSetIfNotExistsInt64, StoreSetIfNotExistsString, StoreSetInt64,
    StoreSetString,
};
use substreams::Hex;
use substreams_database_change::{
    pb::sf::substreams::sink::database::v1::DatabaseChanges, tables::Tables,
};
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

#[allow(unused_imports)] // Might not be needed depending on actual ABI, hence the allow
use {num_traits::cast::ToPrimitive, std::str::FromStr, substreams::scalar::BigDecimal};

substreams_ethereum::init!();

const PUMP_TRACKED_CONTRACT: [u8; 20] = hex!("6c75e165e52e9c1661a75041650be2d919ee02a1");
const IPSHARE_TRACKED_CONTRACT: [u8; 20] = hex!("8a7b0d80fa92699ce3e5bb2c8fe404d6733796d1");
const SWAP_HOOK_CONTRACT: [u8; 20] = hex!("5e8e2d77ce0d2e04ba058bbcecc13c7c8adb20cc");
const CL_POOL_MANAGER: [u8; 20] = hex!("8366a39cc670b4001a1121b8f6a443a643e40951");
const SWAP_TOPIC: [u8; 32] =
    hex!("40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f");
const WALNUT_COMMUNITY_FACTORY: [u8; 20] = hex!("24328dcca1ba54eee82e2993f021802e64290486");
const WALNUT_STAKING_FACTORY: [u8; 20] = hex!("7df32f7a177bcfe437a040579e3bea89dc99c023");
const WALNUT_LOCKING_FACTORY: [u8; 20] = hex!("4ca57c64dfe1cf1be977093c75f9d9cdd1dd2e10");
const WALNUT_SOCIAL_FACTORY: [u8; 20] = hex!("ddbaBa530728b5b8939d7fddc334432490916e90");
const BASKET_REGISTRY: [u8; 20] = hex!("121561caaffcc1f489f8f60e7f7529fd8c1c394b");
const BASKET_HOOK: [u8; 20] = hex!("7c5f5c4358dad036cbbdde6569cbb9cec5b86a88");
const BASKET_ROUTER: [u8; 20] = hex!("e07514fc71bcb16e6c045c937524099ae029d39b");
const BASKET_FEE_AUCTION: [u8; 20] = hex!("17896d2f09b220a7c1a59473708320b8028dc318");
const BASKET_REBALANCE_EXECUTOR: [u8; 20] = hex!("d3f321e93645db6a9e4948ca1198d5bf7d811108");

fn map_pump_events(blk: &eth::Block, events: &mut contract::Events) {
    for rcpt in blk.receipts() {
        for log in rcpt
            .receipt
            .logs
            .iter()
            .filter(|log| log.address == PUMP_TRACKED_CONTRACT)
        {
            if let Some(event) = abi::pump_contract::events::CreateFeeChanged::match_and_decode(log)
            {
                events
                    .pump_create_fee_changeds
                    .push(contract::PumpCreateFeeChanged {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        new_fee: event.new_fee.to_string(),
                        old_fee: event.old_fee.to_string(),
                    });
                continue;
            }
            if let Some(event) =
                abi::pump_contract::events::FeeAddressChanged::match_and_decode(log)
            {
                events
                    .pump_fee_address_changeds
                    .push(contract::PumpFeeAddressChanged {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        new_address: event.new_address,
                        old_address: event.old_address,
                    });
                continue;
            }
            if let Some(event) = abi::pump_contract::events::FeeRatiosChanged::match_and_decode(log)
            {
                events
                    .pump_fee_ratios_changeds
                    .push(contract::PumpFeeRatiosChanged {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        donut_fee: event.donut_fee.to_string(),
                        sellsman_fee: event.sellsman_fee.to_string(),
                    });
                continue;
            }
            if let Some(event) = abi::pump_contract::events::IpShareChanged::match_and_decode(log) {
                events
                    .pump_ip_share_changeds
                    .push(contract::PumpIpShareChanged {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        new_ip_share: event.new_ip_share,
                        old_ip_share: event.old_ip_share,
                    });
                continue;
            }
            if let Some(event) = abi::pump_contract::events::NewToken::match_and_decode(log) {
                events.pump_new_tokens.push(contract::PumpNewToken {
                    evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                    evt_index: log.block_index,
                    evt_block_time: Some(blk.timestamp().to_owned()),
                    evt_block_number: blk.number,
                    creator: event.creator,
                    tick: event.tick,
                    token: event.token,
                    evt_ordinal: log.ordinal,
                    evt_block_hash: blk.hash.clone(),
                });
                continue;
            }
            if let Some(event) =
                abi::pump_contract::events::NutboxAllocationParked::match_and_decode(log)
            {
                events
                    .pump_nutbox_allocation_parkeds
                    .push(contract::PumpNutboxAllocationParked {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        amount: event.amount.to_string(),
                        hook: event.hook,
                        token: event.token,
                    });
                continue;
            }
            if let Some(event) = abi::pump_contract::events::NutboxLinked::match_and_decode(log) {
                events.pump_nutbox_linkeds.push(contract::PumpNutboxLinked {
                    evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                    evt_index: log.block_index,
                    evt_block_time: Some(blk.timestamp().to_owned()),
                    evt_block_number: blk.number,
                    community: event.community,
                    social_pool: event.social_pool,
                    token: event.token,
                });
                continue;
            }
            if let Some(event) =
                abi::pump_contract::events::OwnershipTransferStarted::match_and_decode(log)
            {
                events.pump_ownership_transfer_starteds.push(
                    contract::PumpOwnershipTransferStarted {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        new_owner: event.new_owner,
                        previous_owner: event.previous_owner,
                    },
                );
                continue;
            }
            if let Some(event) =
                abi::pump_contract::events::OwnershipTransferred::match_and_decode(log)
            {
                events
                    .pump_ownership_transferreds
                    .push(contract::PumpOwnershipTransferred {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().to_owned()),
                        evt_block_number: blk.number,
                        new_owner: event.new_owner,
                        previous_owner: event.previous_owner,
                    });
                continue;
            }
        }
    }
}
#[substreams::handlers::map]
fn map_events(blk: eth::Block) -> Result<contract::Events, substreams::errors::Error> {
    let mut events = contract::Events::default();
    map_pump_events(&blk, &mut events);
    Ok(events)
}

#[substreams::handlers::map]
fn map_basket_registry_events(
    blk: eth::Block,
) -> Result<contract::BasketRegistryEvents, substreams::errors::Error> {
    let mut output = contract::BasketRegistryEvents::default();
    for rcpt in blk.receipts() {
        for log in rcpt
            .receipt
            .logs
            .iter()
            .filter(|log| log.address == BASKET_REGISTRY)
        {
            let Some(event) = abi::basket_registry::events::BasketRegistered::match_and_decode(log)
            else {
                continue;
            };
            let salt = rcpt
                .receipt
                .logs
                .iter()
                .filter(|candidate| candidate.address == BASKET_HOOK)
                .filter_map(abi::basket_hook::events::BasketCreated::match_and_decode)
                .find(|created| created.basket == event.basket)
                .map(|created| created.salt.to_vec())
                .unwrap_or_default();
            output.creations.push(contract::BasketCreated {
                evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                evt_index: log.block_index,
                evt_block_time: Some(blk.timestamp().to_owned()),
                evt_block_number: blk.number,
                evt_ordinal: log.ordinal,
                basket: event.basket,
                creator: event.creator,
                registrar: event.registrar,
                version: event.version.to_i32().max(0) as u32,
                created_at: event.created_at.to_u64(),
                salt,
                evt_block_hash: blk.hash.clone(),
            });
        }
    }
    Ok(output)
}

#[substreams::handlers::store]
fn store_basket_addresses(events: contract::BasketRegistryEvents, store: StoreSetInt64) {
    for event in events.creations {
        store.set(event.evt_ordinal, token_key(&event.basket), &1);
    }
}

#[derive(Default)]
struct BasketRouterTrade {
    basket: Vec<u8>,
    payer: Vec<u8>,
    recipient: Vec<u8>,
    usdg_amount: String,
    basket_amount: String,
    is_buy: bool,
    log_index: u32,
    matched: bool,
}

fn take_router_trade(
    candidates: &mut [BasketRouterTrade],
    hook_log_index: u32,
    basket: &[u8],
    is_buy: bool,
    usdg_amount: &str,
    basket_amount: &str,
) -> Option<(Vec<u8>, Vec<u8>, u32)> {
    let candidate = candidates
        .iter_mut()
        .filter(|candidate| {
            !candidate.matched
                && candidate.log_index > hook_log_index
                && candidate.basket == basket
                && candidate.is_buy == is_buy
                && candidate.usdg_amount == usdg_amount
                && candidate.basket_amount == basket_amount
        })
        .min_by_key(|candidate| candidate.log_index)?;
    candidate.matched = true;
    Some((
        candidate.payer.clone(),
        candidate.recipient.clone(),
        candidate.log_index,
    ))
}

#[substreams::handlers::map]
fn map_basket_events(
    blk: eth::Block,
    discoveries: contract::BasketRegistryEvents,
    basket_addresses: StoreGetInt64,
) -> Result<contract::BasketEvents, substreams::errors::Error> {
    let mut output = contract::BasketEvents::default();
    for rcpt in blk.receipts() {
        let tx_hash = Hex(&rcpt.transaction.hash).to_string();
        let mut router_trades = Vec::<BasketRouterTrade>::new();
        for log in rcpt
            .receipt
            .logs
            .iter()
            .filter(|log| log.address == BASKET_ROUTER)
        {
            if let Some(event) = abi::basket_router::events::BasketBought::match_and_decode(log) {
                router_trades.push(BasketRouterTrade {
                    basket: event.basket,
                    payer: event.payer,
                    recipient: event.recipient,
                    usdg_amount: event.usdg_in.to_string(),
                    basket_amount: event.basket_out.to_string(),
                    is_buy: true,
                    log_index: log.block_index,
                    matched: false,
                });
            } else if let Some(event) =
                abi::basket_router::events::BasketCreatedAndBought::match_and_decode(log)
            {
                router_trades.push(BasketRouterTrade {
                    basket: event.basket,
                    payer: event.creator,
                    recipient: event.recipient,
                    usdg_amount: event.usdg_in.to_string(),
                    basket_amount: event.basket_out.to_string(),
                    is_buy: true,
                    log_index: log.block_index,
                    matched: false,
                });
            } else if let Some(event) =
                abi::basket_router::events::BasketSold::match_and_decode(log)
            {
                router_trades.push(BasketRouterTrade {
                    basket: event.basket,
                    payer: event.payer,
                    recipient: event.recipient,
                    usdg_amount: event.usdg_out.to_string(),
                    basket_amount: event.basket_in.to_string(),
                    is_buy: false,
                    log_index: log.block_index,
                    matched: false,
                });
            }
        }

        for log in &rcpt.receipt.logs {
            let common_time = Some(blk.timestamp().to_owned());
            if log.address == BASKET_HOOK {
                let decoded = if let Some(event) =
                    abi::basket_hook::events::BasketBought::match_and_decode(log)
                {
                    Some((
                        event.basket,
                        true,
                        event.usdg_in.to_string(),
                        event.basket_out.to_string(),
                        event.fee_weth.to_string(),
                        event.frontend,
                    ))
                } else if let Some(event) =
                    abi::basket_hook::events::BasketSold::match_and_decode(log)
                {
                    Some((
                        event.basket,
                        false,
                        event.usdg_out.to_string(),
                        event.basket_in.to_string(),
                        event.fee_weth.to_string(),
                        event.frontend,
                    ))
                } else {
                    None
                };
                if let Some((basket, is_buy, usdg_amount, basket_amount, fee_weth, frontend)) =
                    decoded
                {
                    let (payer, recipient, routed, router_evt_index) = take_router_trade(
                        &mut router_trades,
                        log.block_index,
                        &basket,
                        is_buy,
                        &usdg_amount,
                        &basket_amount,
                    )
                    .map(|(payer, recipient, index)| (payer, recipient, true, index))
                    .unwrap_or_else(|| (rcpt.transaction.from.clone(), Vec::new(), false, 0));
                    output.trades.push(contract::BasketTrade {
                        evt_tx_hash: tx_hash.clone(),
                        evt_index: log.block_index,
                        evt_block_time: common_time,
                        evt_block_number: blk.number,
                        evt_ordinal: log.ordinal,
                        basket,
                        is_buy,
                        payer,
                        recipient,
                        frontend,
                        usdg_amount,
                        basket_amount,
                        fee_weth,
                        routed,
                        router_evt_index,
                        evt_block_hash: blk.hash.clone(),
                    });
                    continue;
                }
                if let Some(event) = abi::basket_hook::events::SellLegParked::match_and_decode(log)
                {
                    output.operations.push(contract::BasketOperation {
                        kind: "SELL_LEG_PARKED".into(),
                        evt_tx_hash: tx_hash.clone(),
                        evt_index: log.block_index,
                        evt_block_time: common_time,
                        evt_block_number: blk.number,
                        evt_ordinal: log.ordinal,
                        basket: event.basket,
                        asset: event.asset,
                        amount: event.amount.to_string(),
                        evt_block_hash: blk.hash.clone(),
                        ..Default::default()
                    });
                    continue;
                }
                if let Some(event) =
                    abi::basket_hook::events::PendingBurnSwept::match_and_decode(log)
                {
                    output.operations.push(contract::BasketOperation {
                        kind: "PENDING_BURN_SWEPT".into(),
                        evt_tx_hash: tx_hash.clone(),
                        evt_index: log.block_index,
                        evt_block_time: common_time,
                        evt_block_number: blk.number,
                        evt_ordinal: log.ordinal,
                        basket: event.basket,
                        amount: event.amount.to_string(),
                        evt_block_hash: blk.hash.clone(),
                        ..Default::default()
                    });
                    continue;
                }
            }

            if log.address == BASKET_FEE_AUCTION {
                let mut item = contract::BasketAuctionEvent {
                    evt_tx_hash: tx_hash.clone(),
                    evt_index: log.block_index,
                    evt_block_time: common_time,
                    evt_block_number: blk.number,
                    evt_ordinal: log.ordinal,
                    evt_block_hash: blk.hash.clone(),
                    ..Default::default()
                };
                if let Some(event) =
                    abi::basket_auction::events::AuctionCreated::match_and_decode(log)
                {
                    item.kind = "AUCTION_CREATED".into();
                    item.auction_id = event.auction_id.to_string();
                    item.account = event.creator;
                    item.amount = event.eth_amount.to_string();
                    item.spot_quote = event.spot_quote.to_string();
                    item.initial_bid = event.initial_bid.to_string();
                    item.end_time = event.end_time.to_u64();
                } else if let Some(event) =
                    abi::basket_auction::events::BidPlaced::match_and_decode(log)
                {
                    item.kind = "BID_PLACED".into();
                    item.auction_id = event.auction_id.to_string();
                    item.account = event.bidder;
                    item.amount = event.total_bid.to_string();
                } else if let Some(event) =
                    abi::basket_auction::events::AuctionSettled::match_and_decode(log)
                {
                    item.kind = "AUCTION_SETTLED".into();
                    item.auction_id = event.auction_id.to_string();
                    item.account = event.winner;
                    item.amount = event.eth_amount.to_string();
                    item.secondary_amount = event.bid_token_burned.to_string();
                } else if let Some(event) =
                    abi::basket_auction::events::EthClaimed::match_and_decode(log)
                {
                    item.kind = "ETH_CLAIMED".into();
                    item.account = event.winner;
                    item.recipient = event.recipient;
                    item.amount = event.amount.to_string();
                } else if let Some(event) =
                    abi::basket_auction::events::FundsReceived::match_and_decode(log)
                {
                    item.kind = "FUNDS_RECEIVED".into();
                    item.account = event.sender;
                    item.amount = event.amount.to_string();
                } else if let Some(event) =
                    abi::basket_auction::events::BidTokensDeposited::match_and_decode(log)
                {
                    item.kind = "BID_TOKENS_DEPOSITED".into();
                    item.account = event.bidder;
                    item.amount = event.amount.to_string();
                } else if let Some(event) =
                    abi::basket_auction::events::BidTokensWithdrawn::match_and_decode(log)
                {
                    item.kind = "BID_TOKENS_WITHDRAWN".into();
                    item.account = event.bidder;
                    item.recipient = event.recipient;
                    item.amount = event.amount.to_string();
                } else {
                    continue;
                }
                output.auction_events.push(item);
                continue;
            }

            if log.address == BASKET_REBALANCE_EXECUTOR {
                if let Some(event) =
                    abi::basket_rebalance::events::BasketRebalanced::match_and_decode(log)
                {
                    output.rebalances.push(contract::BasketRebalance {
                        evt_tx_hash: tx_hash.clone(),
                        evt_index: log.block_index,
                        evt_block_time: common_time,
                        evt_block_number: blk.number,
                        evt_ordinal: log.ordinal,
                        basket: event.basket,
                        nav_before: event.nav_before.to_string(),
                        nav_after: event.nav_after.to_string(),
                        evt_block_hash: blk.hash.clone(),
                    });
                }
                continue;
            }

            let address_key = token_key(&log.address);
            let is_basket = basket_addresses.get_at(log.ordinal, &address_key).is_some()
                || discoveries
                    .creations
                    .iter()
                    .any(|event| event.basket == log.address);
            if !is_basket {
                continue;
            }
            if let Some(event) = abi::basket_token::events::FeeAccrued::match_and_decode(log) {
                output.fee_accruals.push(contract::BasketFeeAccrual {
                    evt_tx_hash: tx_hash.clone(),
                    evt_index: log.block_index,
                    evt_block_time: common_time,
                    evt_block_number: blk.number,
                    evt_ordinal: log.ordinal,
                    basket: log.address.clone(),
                    holder_amount: event.holder_amount.to_string(),
                    auction_amount: event.auction_amount.to_string(),
                    creator_amount: event.creator_amount.to_string(),
                    launcher_amount: event.launcher_amount.to_string(),
                    frontend: event.frontend,
                    frontend_amount: event.frontend_amount.to_string(),
                    evt_block_hash: blk.hash.clone(),
                });
                continue;
            }
            let claim = if let Some(event) =
                abi::basket_token::events::HolderFeesClaimed::match_and_decode(log)
            {
                Some(("HOLDER", event.holder, event.amount.to_string()))
            } else if let Some(event) =
                abi::basket_token::events::CreatorFeesClaimed::match_and_decode(log)
            {
                Some(("CREATOR", event.recipient, event.amount.to_string()))
            } else if let Some(event) =
                abi::basket_token::events::LauncherFeesClaimed::match_and_decode(log)
            {
                Some(("LAUNCHER", event.recipient, event.amount.to_string()))
            } else if let Some(event) =
                abi::basket_token::events::FrontendFeesClaimed::match_and_decode(log)
            {
                Some(("FRONTEND", event.frontend, event.amount.to_string()))
            } else {
                None
            };
            if let Some((claim_type, beneficiary, amount)) = claim {
                output.fee_claims.push(contract::BasketFeeClaim {
                    claim_type: claim_type.into(),
                    evt_tx_hash: tx_hash.clone(),
                    evt_index: log.block_index,
                    evt_block_time: common_time,
                    evt_block_number: blk.number,
                    evt_ordinal: log.ordinal,
                    basket: log.address.clone(),
                    beneficiary,
                    amount,
                    evt_block_hash: blk.hash.clone(),
                });
                continue;
            }
            if let Some(event) = abi::basket_token::events::RedeemedInKind::match_and_decode(log) {
                output.operations.push(contract::BasketOperation {
                    kind: "REDEEMED_IN_KIND".into(),
                    evt_tx_hash: tx_hash.clone(),
                    evt_index: log.block_index,
                    evt_block_time: common_time,
                    evt_block_number: blk.number,
                    evt_ordinal: log.ordinal,
                    basket: log.address.clone(),
                    account: event.holder,
                    recipient: event.recipient,
                    amount: event.shares_burned.to_string(),
                    evt_block_hash: blk.hash.clone(),
                    ..Default::default()
                });
            }
        }
    }
    Ok(output)
}

#[substreams::handlers::map]
fn map_ipshare_events(
    blk: eth::Block,
) -> Result<contract::IpShareEvents, substreams::errors::Error> {
    let mut events = contract::IpShareEvents::default();
    for rcpt in blk.receipts() {
        for log in rcpt
            .receipt
            .logs
            .iter()
            .filter(|log| log.address == IPSHARE_TRACKED_CONTRACT)
        {
            let common = || {
                (
                    Hex(&rcpt.transaction.hash).to_string(),
                    log.block_index,
                    Some(blk.timestamp().to_owned()),
                    blk.number,
                    log.ordinal,
                    blk.hash.clone(),
                )
            };
            if let Some(event) = abi::ipshare_contract::events::CreateIPshare::match_and_decode(log)
            {
                let (tx, index, time, block, ordinal, block_hash) = common();
                events.creates.push(contract::IpShareCreate {
                    evt_tx_hash: tx,
                    evt_index: index,
                    evt_block_time: time,
                    evt_block_number: block,
                    subject: event.subject,
                    amount: event.amount.to_string(),
                    create_fee: event.create_fee.to_string(),
                    evt_ordinal: ordinal,
                    evt_block_hash: block_hash,
                });
                continue;
            }
            if let Some(event) = abi::ipshare_contract::events::Trade::match_and_decode(log) {
                let (tx, index, time, block, ordinal, block_hash) = common();
                events.trades.push(contract::IpShareTrade {
                    evt_tx_hash: tx,
                    evt_index: index,
                    evt_block_time: time,
                    evt_block_number: block,
                    trader: event.trader,
                    subject: event.subject,
                    is_buy: event.is_buy,
                    share_amount: event.share_amount.to_string(),
                    eth_amount: event.eth_amount.to_string(),
                    protocol_eth_amount: event.protocol_eth_amount.to_string(),
                    subject_eth_amount: event.subject_eth_amount.to_string(),
                    supply: event.supply.to_string(),
                    evt_ordinal: ordinal,
                    evt_block_hash: block_hash,
                });
                continue;
            }
            if let Some(event) = abi::ipshare_contract::events::ValueCaptured::match_and_decode(log)
            {
                let (tx, index, time, block, ordinal, block_hash) = common();
                events.value_captures.push(contract::IpShareValueCaptured {
                    evt_tx_hash: tx,
                    evt_index: index,
                    evt_block_time: time,
                    evt_block_number: block,
                    subject: event.subject,
                    investor: event.investor,
                    amount: event.amount.to_string(),
                    evt_ordinal: ordinal,
                    evt_block_hash: block_hash,
                });
                continue;
            }
            if let Some(event) = abi::ipshare_contract::events::Stake::match_and_decode(log) {
                let (tx, index, time, block, ordinal, block_hash) = common();
                events.stakes.push(contract::IpShareStake {
                    evt_tx_hash: tx,
                    evt_index: index,
                    evt_block_time: time,
                    evt_block_number: block,
                    staker: event.staker,
                    subject: event.subject,
                    is_stake: event.is_stake,
                    amount: event.amount.to_string(),
                    staked_amount: event.staked_amount.to_string(),
                    evt_ordinal: ordinal,
                    evt_block_hash: block_hash,
                });
            }
        }
    }
    Ok(events)
}

#[substreams::handlers::map]
fn map_walnut_factory_events(
    blk: eth::Block,
) -> Result<contract::WalnutEvents, substreams::errors::Error> {
    let mut output = contract::WalnutEvents::default();
    for rcpt in blk.receipts() {
        for log in &rcpt.receipt.logs {
            let mut item = contract::WalnutEvent {
                evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                evt_index: log.block_index,
                evt_block_time: Some(blk.timestamp().to_owned()),
                evt_block_number: blk.number,
                evt_ordinal: log.ordinal,
                contract: log.address.clone(),
                operator: rcpt.transaction.from.clone(),
                evt_block_hash: blk.hash.clone(),
                ..Default::default()
            };
            if log.address == WALNUT_COMMUNITY_FACTORY {
                if let Some(e) = abi::walnut::events::CommunityCreated::match_and_decode(log) {
                    item.kind = "COMMUNITY_CREATED".into();
                    item.community = e.community;
                    item.pool = e.community_token;
                    item.account = e.creator;
                    output.events.push(item);
                }
            } else if log.address == WALNUT_STAKING_FACTORY {
                if let Some(e) = abi::walnut::events::Erc20StakingCreated::match_and_decode(log) {
                    item.kind = "ERC20_STAKING_CREATED".into();
                    item.pool = e.pool;
                    item.community = e.community;
                    item.name = e.name;
                    item.asset = e.erc20_token;
                    output.events.push(item);
                }
            } else if log.address == WALNUT_LOCKING_FACTORY {
                if let Some(e) = abi::walnut::events::Erc20LockingCreated::match_and_decode(log) {
                    item.kind = "ERC20_LOCKING_CREATED".into();
                    item.pool = e.pool;
                    item.community = e.community;
                    item.name = e.name;
                    item.asset = e.erc20_token;
                    item.amount = e.lock_duration.to_string();
                    output.events.push(item);
                }
            } else if log.address == WALNUT_SOCIAL_FACTORY {
                if let Some(e) = abi::walnut::events::SocialCurationCreated::match_and_decode(log) {
                    item.kind = "SOCIAL_CURATION_CREATED".into();
                    item.pool = e.pool;
                    item.community = e.community;
                    item.name = e.name;
                    output.events.push(item);
                }
            }
        }
    }
    Ok(output)
}

#[substreams::handlers::store]
fn store_walnut_contracts(events: contract::WalnutEvents, store: StoreSetString) {
    for event in events.events {
        if event.kind == "COMMUNITY_CREATED" {
            store.set(
                event.evt_ordinal,
                prefixed_hex(&event.community),
                &format!("COMMUNITY|{}", prefixed_hex(&event.pool)),
            );
        } else if event.kind.ends_with("_CREATED") {
            store.set(
                event.evt_ordinal,
                prefixed_hex(&event.pool),
                &format!(
                    "{}|{}|{}|{}",
                    event.kind,
                    prefixed_hex(&event.community),
                    prefixed_hex(&event.asset),
                    prefixed_hex(&event.contract)
                ),
            );
        }
    }
}

#[substreams::handlers::store]
fn store_walnut_owners(
    factory: contract::WalnutEvents,
    dynamic: contract::WalnutEvents,
    store: StoreSetString,
) {
    let mut changes: Vec<(u64, String, String)> = Vec::new();
    for event in factory.events {
        if event.kind == "COMMUNITY_CREATED" {
            changes.push((
                event.evt_ordinal,
                prefixed_hex(&event.community),
                prefixed_hex(&event.account),
            ));
        }
    }
    for event in dynamic.events {
        if event.kind == "OWNERSHIP_TRANSFERRED" {
            changes.push((
                event.evt_ordinal,
                prefixed_hex(&event.community),
                prefixed_hex(&event.account),
            ));
        }
    }
    changes.sort_by_key(|change| change.0);
    for (ordinal, community, owner) in changes {
        store.set(ordinal, community, &owner);
    }
}

#[substreams::handlers::store]
fn store_walnut_memberships(
    factory: contract::WalnutEvents,
    dynamic: contract::WalnutEvents,
    store: StoreSetIfNotExistsInt64,
) {
    let mut memberships: Vec<(u64, String, i64)> = Vec::new();
    for event in factory.events {
        if event.kind == "COMMUNITY_CREATED" {
            push_walnut_user_memberships(
                &mut memberships,
                event.evt_ordinal,
                event_timestamp(event.evt_block_time),
                &event.community,
                &event.account,
            );
        }
    }
    for event in dynamic.events {
        let timestamp = event_timestamp(event.evt_block_time.clone());
        let creates_user = matches!(event.kind.as_str(), "DEPOSIT" | "LOCK" | "SOCIALCLAIM")
            || (event.kind == "OWNERSHIP_TRANSFERRED"
                && !is_zero_address(&event.secondary_account));
        if creates_user {
            push_walnut_user_memberships(
                &mut memberships,
                event.evt_ordinal,
                timestamp,
                &event.community,
                &event.account,
            );
        }
        if matches!(event.kind.as_str(), "DEPOSIT" | "LOCK" | "SOCIALCLAIM") {
            memberships.push((
                event.evt_ordinal,
                format!(
                    "pool|{}|{}",
                    prefixed_hex(&event.pool),
                    prefixed_hex(&event.account)
                ),
                timestamp,
            ));
        }
        if matches!(event.kind.as_str(), "DEPOSIT" | "LOCK") {
            memberships.push((
                event.evt_ordinal,
                format!(
                    "staker|{}|{}",
                    prefixed_hex(&event.pool),
                    prefixed_hex(&event.account)
                ),
                timestamp,
            ));
        }
    }
    memberships.sort_by_key(|membership| membership.0);
    for (ordinal, key, timestamp) in memberships {
        store.set_if_not_exists(ordinal, key, &timestamp);
    }
}

fn push_walnut_user_memberships(
    memberships: &mut Vec<(u64, String, i64)>,
    ordinal: u64,
    timestamp: i64,
    community: &[u8],
    account: &[u8],
) {
    let account = prefixed_hex(account);
    memberships.push((
        ordinal,
        format!("community|{}|{account}", prefixed_hex(community)),
        timestamp,
    ));
}

#[substreams::handlers::store]
fn store_pre_walnut_accounts(
    ipshare: contract::IpShareEvents,
    pump: contract::Events,
    swaps: contract::TokenEvents,
    store: StoreSetIfNotExistsString,
) {
    let mut candidates: Vec<(u8, u64, String, i64)> = Vec::new();
    let mut push = |priority: u8, ordinal: u64, address: &[u8], timestamp: i64| {
        candidates.push((priority, ordinal, prefixed_hex(address), timestamp));
    };
    for event in ipshare.creates {
        push(
            0,
            event.evt_ordinal,
            &event.subject,
            event_timestamp(event.evt_block_time),
        );
    }
    for event in ipshare.trades {
        let timestamp = event_timestamp(event.evt_block_time);
        push(0, event.evt_ordinal, &event.trader, timestamp);
        push(0, event.evt_ordinal, &event.subject, timestamp);
    }
    for event in ipshare.value_captures {
        let timestamp = event_timestamp(event.evt_block_time);
        push(0, event.evt_ordinal, &event.subject, timestamp);
        push(0, event.evt_ordinal, &event.investor, timestamp);
    }
    for event in ipshare.stakes {
        let timestamp = event_timestamp(event.evt_block_time);
        push(0, event.evt_ordinal, &event.staker, timestamp);
        push(0, event.evt_ordinal, &event.subject, timestamp);
    }
    for event in pump.pump_new_tokens {
        push(
            1,
            event.evt_ordinal,
            &event.creator,
            event_timestamp(event.evt_block_time),
        );
    }
    for event in swaps.swap_trades {
        let timestamp = event_timestamp(event.evt_block_time);
        push(2, event.evt_ordinal, &event.buyer, timestamp);
        push(2, event.evt_ordinal, &event.sellsman, timestamp);
    }
    candidates.sort_by_key(|candidate| (candidate.0, candidate.1));
    let mut selected: Vec<(u64, String, i64)> = Vec::new();
    for (_, ordinal, account, timestamp) in candidates {
        if selected.iter().any(|(_, existing, _)| existing == &account) {
            continue;
        }
        selected.push((ordinal, account, timestamp));
    }
    selected.sort_by_key(|candidate| candidate.0);
    for (ordinal, account, timestamp) in selected {
        store.set_if_not_exists(ordinal, account, &timestamp.to_string());
    }
}

#[substreams::handlers::store]
fn store_walnut_accounts(
    walnut_factory: contract::WalnutEvents,
    walnut_dynamic: contract::WalnutEvents,
    pre_walnut_accounts: StoreGetString,
    store: StoreSetIfNotExistsString,
) {
    let mut candidates: Vec<(u8, u64, String, i64)> = Vec::new();
    let mut push = |priority: u8, ordinal: u64, address: &[u8], timestamp: i64| {
        candidates.push((priority, ordinal, prefixed_hex(address), timestamp));
    };
    for event in walnut_factory.events {
        if event.kind == "COMMUNITY_CREATED" {
            push(
                1,
                event.evt_ordinal,
                &event.account,
                event_timestamp(event.evt_block_time),
            );
        }
    }
    for event in walnut_dynamic.events {
        if matches!(event.kind.as_str(), "DEPOSIT" | "LOCK" | "SOCIALCLAIM")
            || (event.kind == "OWNERSHIP_TRANSFERRED" && !is_zero_address(&event.secondary_account))
        {
            push(
                2,
                event.evt_ordinal,
                &event.account,
                event_timestamp(event.evt_block_time),
            );
        }
    }
    candidates.sort_by_key(|candidate| (candidate.0, candidate.1));
    let mut selected: Vec<(u64, String, String)> = Vec::new();
    for (priority, ordinal, account, timestamp) in candidates {
        if selected.iter().any(|(_, existing, _)| existing == &account) {
            continue;
        }
        if pre_walnut_accounts.get_last(&account).is_some() {
            continue;
        }
        selected.push((ordinal, account, format!("{timestamp}|{priority}")));
    }
    selected.sort_by_key(|candidate| candidate.0);
    for (ordinal, account, value) in selected {
        store.set_if_not_exists(ordinal, account, &value);
    }
}

#[substreams::handlers::store]
fn store_token_accounts(
    events: contract::TokenEvents,
    pre_walnut_accounts: StoreGetString,
    walnut_accounts: StoreGetString,
    store: StoreSetIfNotExistsString,
) {
    let mut candidates: Vec<(u64, String, i64)> = Vec::new();
    for event in events.trades {
        let timestamp = event_timestamp(event.evt_block_time);
        candidates.push((event.evt_ordinal, prefixed_hex(&event.buyer), timestamp));
        candidates.push((event.evt_ordinal, prefixed_hex(&event.sellsman), timestamp));
    }
    candidates.sort_by_key(|candidate| candidate.0);
    let mut selected: Vec<(u64, String, i64)> = Vec::new();
    for candidate in candidates {
        if selected
            .iter()
            .any(|(_, account, _)| account == &candidate.1)
        {
            continue;
        }
        if pre_walnut_accounts.get_last(&candidate.1).is_some()
            || walnut_accounts.get_last(&candidate.1).is_some()
        {
            continue;
        }
        selected.push(candidate);
    }
    for (ordinal, account, timestamp) in selected {
        store.set_if_not_exists(ordinal, account, &timestamp.to_string());
    }
}

#[substreams::handlers::store]
fn store_user_account_indexes(
    pre_walnut_accounts: Deltas<DeltaString>,
    token_accounts: Deltas<DeltaString>,
    store: StoreAddInt64,
) {
    let mut ordinals: Vec<u64> = pre_walnut_accounts
        .deltas
        .into_iter()
        .chain(token_accounts.deltas)
        .map(|delta| delta.ordinal)
        .collect();
    ordinals.sort_unstable();
    for ordinal in ordinals {
        store.add(ordinal, "user", 1);
    }
}

#[substreams::handlers::store]
fn store_walnut_account_indexes(accounts: Deltas<DeltaString>, store: StoreAddInt64) {
    for delta in accounts.deltas {
        store.add(delta.ordinal, "walnut_user", 1);
    }
}

#[substreams::handlers::store]
fn store_walnut_indexes(
    factory: contract::WalnutEvents,
    dynamic: contract::WalnutEvents,
    store: StoreAddInt64,
) {
    let mut changes: Vec<(u64, &'static str)> = Vec::new();
    for event in factory.events {
        if event.kind == "COMMUNITY_CREATED" {
            changes.push((event.evt_ordinal, "walnut_community"));
        } else {
            changes.push((event.evt_ordinal, "walnut_pool"));
        }
        changes.push((event.evt_ordinal, "walnut_op"));
    }
    for event in dynamic.events {
        if !matches!(
            event.kind.as_str(),
            "POOL_UPDATED" | "OWNERSHIP_TRANSFERRED"
        ) {
            changes.push((event.evt_ordinal, "walnut_op"));
        }
    }
    changes.sort_by_key(|change| change.0);
    for (ordinal, key) in changes {
        store.add(ordinal, key, 1);
    }
}

#[substreams::handlers::map]
fn map_walnut_events(
    blk: eth::Block,
    factory_events: contract::WalnutEvents,
    contracts: StoreGetString,
) -> Result<contract::WalnutEvents, substreams::errors::Error> {
    let mut output = contract::WalnutEvents::default();
    for rcpt in blk.receipts() {
        for log in &rcpt.receipt.logs {
            let address = prefixed_hex(&log.address);
            let descriptor = contracts.get_at(log.ordinal, &address).or_else(|| {
                factory_events
                    .events
                    .iter()
                    .find(|event| {
                        if event.kind == "COMMUNITY_CREATED" {
                            event.community == log.address
                        } else {
                            event.pool == log.address
                        }
                    })
                    .map(walnut_descriptor)
            });
            let Some(descriptor) = descriptor else {
                continue;
            };
            let parts: Vec<&str> = descriptor.split('|').collect();
            let contract_type = parts[0];
            let mut item = contract::WalnutEvent {
                evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                evt_index: log.block_index,
                evt_block_time: Some(blk.timestamp().to_owned()),
                evt_block_number: blk.number,
                evt_ordinal: log.ordinal,
                contract: log.address.clone(),
                operator: rcpt.transaction.from.clone(),
                evt_block_hash: blk.hash.clone(),
                ..Default::default()
            };

            if contract_type == "COMMUNITY" {
                item.community = log.address.clone();
                if let Some(e) = abi::walnut::events::AdminSetFeeRatio::match_and_decode(log) {
                    item.kind = "ADMINSETFEE".into();
                    item.ratio = e.ratio.to_i32() as u32;
                    // The legacy admin operation stores the fee ratio in its
                    // generic `amount` field as well as updating Community.
                    item.amount = e.ratio.to_string();
                } else if let Some(e) =
                    abi::walnut::events::AdminSetPoolRatio::match_and_decode(log)
                {
                    item.kind = "ADMINSETRATIO".into();
                    item.pools = e.pools;
                    item.ratios = e.ratios.into_iter().map(|v| v.to_i32() as u32).collect();
                } else if let Some(e) = abi::walnut::events::PoolUpdated::match_and_decode(log) {
                    item.kind = "POOL_UPDATED".into();
                    item.account = e.who;
                    item.amount = e.amount.to_string();
                } else if let Some(e) = abi::walnut::events::AdminClosePool::match_and_decode(log) {
                    item.kind = "ADMINCLOSEPOOL".into();
                    item.pool = e.pool;
                } else if let Some(e) = abi::walnut::events::WithdrawRewards::match_and_decode(log)
                {
                    item.kind = if e.pool.len() == 1 {
                        "HARVEST".into()
                    } else {
                        "HARVESTALL".into()
                    };
                    item.pools = e.pool;
                    item.account = e.who;
                    item.amount = e.amount.to_string();
                    if item.pools.len() == 1 {
                        item.pool = item.pools[0].clone();
                        if let Some(pool_desc) =
                            contracts.get_at(log.ordinal, prefixed_hex(&item.pool))
                        {
                            let pool_parts: Vec<&str> = pool_desc.split('|').collect();
                            if pool_parts.len() >= 4 {
                                item.pool_factory = decode_address(pool_parts[3]);
                            }
                        }
                    }
                    if let Some(community_desc) = contracts.get_at(log.ordinal, &address) {
                        if let Some(token) = community_desc.split('|').nth(1) {
                            item.asset = decode_address(token);
                        }
                    }
                } else if let Some(e) =
                    abi::walnut::events::OwnershipTransferred::match_and_decode(log)
                {
                    item.kind = "OWNERSHIP_TRANSFERRED".into();
                    item.account = e.new_owner;
                    item.secondary_account = e.previous_owner;
                } else if let Some(e) = abi::walnut::events::DevChanged::match_and_decode(log) {
                    item.kind = "ADMINSETDAOFUND".into();
                    item.account = e.new_dev;
                    item.secondary_account = e.old_dev;
                } else if let Some(e) = abi::walnut::events::RevenueWithdrawn::match_and_decode(log)
                {
                    item.kind = "ADMINWITHDRAWNREVENUE".into();
                    item.account = e.dev_fund;
                    item.amount = e.amount.to_string();
                } else {
                    continue;
                }
            } else {
                if parts.len() < 4 {
                    continue;
                }
                item.community = decode_address(parts[1]);
                item.asset = decode_address(parts[2]);
                item.pool_factory = decode_address(parts[3]);
                if contract_type == "ERC20_STAKING_CREATED" {
                    if let Some(e) = abi::walnut::events::Deposited::match_and_decode(log) {
                        item.kind = "DEPOSIT".into();
                        item.account = e.who;
                        item.amount = e.amount.to_string();
                        item.community = e.community;
                    } else if let Some(e) = abi::walnut::events::Withdrawn::match_and_decode(log) {
                        item.kind = "WITHDRAW".into();
                        item.account = e.who;
                        item.amount = e.amount.to_string();
                        item.community = e.community;
                    } else {
                        continue;
                    }
                } else if contract_type == "ERC20_LOCKING_CREATED" {
                    if let Some(e) = abi::walnut::events::Locked::match_and_decode(log) {
                        item.kind = "LOCK".into();
                        item.account = e.who;
                        item.amount = e.amount.to_string();
                    } else if let Some(e) = abi::walnut::events::Unlocked::match_and_decode(log) {
                        item.kind = "UNLOCK".into();
                        item.account = e.who;
                        item.amount = e.amount.to_string();
                    } else if let Some(e) = abi::walnut::events::Redeemed::match_and_decode(log) {
                        item.kind = "REDEEM".into();
                        item.account = e.who;
                        item.amount = e.amount.to_string();
                    } else {
                        continue;
                    }
                } else if contract_type == "SOCIAL_CURATION_CREATED" {
                    let Some(e) = abi::walnut::events::SocialClaimed::match_and_decode(log) else {
                        continue;
                    };
                    item.kind = "SOCIALCLAIM".into();
                    item.account = e.user;
                    item.amount = e.amount.to_string();
                    item.secondary_amount = e.order_id.to_string();
                    item.flag = e.harvested;
                    if let Some(community_desc) = contracts.get_at(log.ordinal, parts[1]) {
                        if let Some(token) = community_desc.split('|').nth(1) {
                            item.asset = decode_address(token);
                        }
                    }
                } else {
                    continue;
                }
                item.pool = log.address.clone();
            }
            output.events.push(item);
        }
    }
    Ok(output)
}

fn walnut_descriptor(event: &contract::WalnutEvent) -> String {
    if event.kind == "COMMUNITY_CREATED" {
        format!("COMMUNITY|{}", prefixed_hex(&event.pool))
    } else {
        format!(
            "{}|{}|{}|{}",
            event.kind,
            prefixed_hex(&event.community),
            prefixed_hex(&event.asset),
            prefixed_hex(&event.contract)
        )
    }
}

fn decode_address(value: &str) -> Vec<u8> {
    hex::decode(value.trim_start_matches("0x")).expect("valid stored Walnut address")
}

#[substreams::handlers::store]
fn store_ipshare_indexes(events: contract::IpShareEvents, store: StoreAddInt64) {
    for event in events.creates {
        store.add(event.evt_ordinal, "ipshare", 1);
    }
    for event in events.trades {
        store.add(event.evt_ordinal, "ipshare_trade", 1);
    }
    for event in events.value_captures {
        store.add(event.evt_ordinal, "value_capture", 1);
    }
    for event in events.stakes {
        store.add(event.evt_ordinal, "stake", 1);
    }
}

#[substreams::handlers::store]
fn store_ipshare_holder_balances(events: contract::IpShareEvents, store: StoreAddBigInt) {
    for event in events.creates {
        let subject = prefixed_hex(&event.subject);
        store.add(
            event.evt_ordinal,
            holder_key(&subject, &subject),
            parse_bigint(&event.amount),
        );
    }
    for event in events.trades {
        let key = holder_key(&prefixed_hex(&event.trader), &prefixed_hex(&event.subject));
        let amount = parse_bigint(&event.share_amount);
        store.add(
            event.evt_ordinal,
            key,
            if event.is_buy { amount } else { -amount },
        );
    }
}

#[substreams::handlers::store]
fn store_ipshare_stake_balances(events: contract::IpShareEvents, store: StoreAddBigInt) {
    for event in events.stakes {
        let key = holder_key(&prefixed_hex(&event.staker), &prefixed_hex(&event.subject));
        let amount = parse_bigint(&event.amount);
        store.add(
            event.evt_ordinal,
            key,
            if event.is_stake { amount } else { -amount },
        );
    }
}

#[substreams::handlers::store]
fn store_token_addresses(events: contract::Events, store: StoreSetInt64) {
    for event in events.pump_new_tokens {
        store.set(event.evt_ordinal, token_key(&event.token), &1);
    }
}

#[substreams::handlers::map]
fn map_swap_events(blk: eth::Block) -> Result<contract::TokenEvents, substreams::errors::Error> {
    let mut output = contract::TokenEvents::default();
    for rcpt in blk.receipts() {
        for hook_log in rcpt
            .receipt
            .logs
            .iter()
            .filter(|log| log.address == SWAP_HOOK_CONTRACT)
        {
            let Some(fee) = abi::swap_hook::events::SwapFeeCollected::match_and_decode(hook_log)
            else {
                continue;
            };
            let Some(swap_log) = rcpt.receipt.logs.iter().find(|log| {
                log.address == CL_POOL_MANAGER
                    && log.topics.first().map(|t| t.as_slice()) == Some(SWAP_TOPIC.as_slice())
                    && log.topics.get(1).map(|t| t.as_slice()) == Some(fee.pool_id.as_slice())
            }) else {
                continue;
            };
            if swap_log.data.len() < 192 {
                continue;
            }
            let amount0 = BigInt::from_signed_bytes_be(&swap_log.data[0..32]);
            let amount1 = BigInt::from_signed_bytes_be(&swap_log.data[32..64]);
            let sqrt_price = BigInt::from_unsigned_bytes_be(&swap_log.data[64..96]);
            let zero = BigInt::from(0);
            let is_buy = amount1 > zero;
            output.swap_trades.push(contract::TokenTrade {
                evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                evt_index: hook_log.block_index,
                evt_block_time: Some(blk.timestamp().to_owned()),
                evt_block_number: blk.number,
                token: fee.token,
                buyer: rcpt.transaction.from.clone(),
                sellsman: vec![0; 20],
                is_buy,
                token_amount: amount1.absolute().to_string(),
                eth_amount: amount0.absolute().to_string(),
                tiptag_fee: fee.platform_fee.to_string(),
                sellsman_fee: fee.deployer_fee.to_string(),
                evt_ordinal: hook_log.ordinal,
                price: swap_price(&sqrt_price).to_string(),
                evt_block_hash: blk.hash.clone(),
            });
        }
    }
    Ok(output)
}

#[substreams::handlers::map]
fn map_token_events(
    blk: eth::Block,
    discoveries: contract::Events,
    token_addresses: StoreGetInt64,
) -> Result<contract::TokenEvents, substreams::errors::Error> {
    let mut events = contract::TokenEvents::default();

    for rcpt in blk.receipts() {
        for log in &rcpt.receipt.logs {
            let key = token_key(&log.address);
            let existed_before_block = token_addresses.has_first(&key);
            let discovered_before_log = discoveries
                .pump_new_tokens
                .iter()
                .any(|event| event.token == log.address && event.evt_ordinal < log.ordinal);

            if !existed_before_block && !discovered_before_log {
                continue;
            }

            if let Some(event) = abi::token_contract::events::Trade::match_and_decode(log) {
                events.trades.push(contract::TokenTrade {
                    evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                    evt_index: log.block_index,
                    evt_block_time: Some(blk.timestamp().to_owned()),
                    evt_block_number: blk.number,
                    token: log.address.clone(),
                    buyer: event.buyer,
                    sellsman: event.sellsman,
                    is_buy: event.is_buy,
                    token_amount: event.token_amount.to_string(),
                    eth_amount: event.eth_amount.to_string(),
                    tiptag_fee: event.tiptag_fee.to_string(),
                    sellsman_fee: event.sellsman_fee.to_string(),
                    evt_ordinal: log.ordinal,
                    price: String::new(),
                    evt_block_hash: blk.hash.clone(),
                });
                continue;
            }

            if let Some(event) =
                abi::token_contract::events::TokenListedToDex::match_and_decode(log)
            {
                events.listed_to_dex.push(contract::TokenListedToDex {
                    evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                    evt_index: log.block_index,
                    evt_block_time: Some(blk.timestamp().to_owned()),
                    evt_block_number: blk.number,
                    token: log.address.clone(),
                    event_token: event.token,
                    pool_id: event.pool_id.to_vec(),
                    sqrt_price_x96: event.sqrt_price_x96.to_string(),
                    evt_ordinal: log.ordinal,
                    evt_block_hash: blk.hash.clone(),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::store]
fn store_bonding_curve_supply(events: contract::TokenEvents, store: StoreAddBigInt) {
    for event in events.trades {
        let amount = BigInt::from_str(&event.token_amount).expect("valid token amount");
        let delta = if event.is_buy { amount } else { -amount };
        store.add(event.evt_ordinal, token_key(&event.token), delta);
    }
}

#[substreams::handlers::store]
fn store_entity_indexes(
    events: contract::Events,
    token_events: contract::TokenEvents,
    swap_events: contract::TokenEvents,
    store: StoreAddInt64,
) {
    for event in events.pump_new_tokens {
        store.add(event.evt_ordinal, "token", 1);
    }
    for event in token_events.trades {
        store.add(event.evt_ordinal, "token_trade", 1);
    }
    for event in swap_events.swap_trades {
        store.add(event.evt_ordinal, "token_trade", 1);
    }
    for event in token_events.listed_to_dex {
        store.add(event.evt_ordinal, "listed_token", 1);
    }
}

#[substreams::handlers::map]
#[allow(deprecated)]
fn graph_out(events: contract::Events) -> Result<EntityChanges, substreams::errors::Error> {
    let mut entity_changes = EntityChanges::default();

    for event in events.pump_new_tokens {
        let token_id = format!("0x{}", Hex(&event.token));
        let creator = format!("0x{}", Hex(&event.creator));
        let timestamp = event
            .evt_block_time
            .map(|value| value.seconds)
            .unwrap_or_default();

        entity_changes.entity_changes.push(EntityChange {
            entity: "PumpTokenDiscovery".to_string(),
            id: token_id,
            ordinal: 0,
            operation: Operation::Create as i32,
            fields: vec![
                field("token", Typed::Bytes(base64_encode(&event.token))),
                field("creator", Typed::Bytes(base64_encode(&event.creator))),
                field("creatorHex", Typed::String(creator)),
                field("symbol", Typed::String(event.tick)),
                field(
                    "blockNumber",
                    Typed::Bigint(event.evt_block_number.to_string()),
                ),
                field("timestamp", Typed::Bigint(timestamp.to_string())),
                field("transaction", Typed::String(event.evt_tx_hash)),
                field("logIndex", Typed::Int32(event.evt_index as i32)),
            ],
        });
    }

    Ok(entity_changes)
}

#[allow(deprecated)]
fn field(name: &str, typed: Typed) -> Field {
    Field {
        name: name.to_string(),
        new_value: Some(Value { typed: Some(typed) }),
        old_value: None,
    }
}

fn write_basket_changes(
    tables: &mut Tables,
    basket_registry_events: contract::BasketRegistryEvents,
    basket_events: contract::BasketEvents,
) {
    for event in basket_registry_events.creations {
        let basket = prefixed_hex(&event.basket);
        tables
            .upsert_row("baskets", &basket)
            .set("creator", prefixed_hex(&event.creator))
            .set("registrar", prefixed_hex(&event.registrar))
            .set("version", event.version)
            .set("created_at", event.created_at)
            .set("salt", prefixed_hex(&event.salt))
            .set("creation_block", event.evt_block_number)
            .set("creation_block_hash", prefixed_hex(&event.evt_block_hash))
            .set("creation_transaction_hash", event.evt_tx_hash)
            .set("creation_log_index", event.evt_index);

        // Basket holders use the same Blockscout refresh worker as TagAI
        // tokens; no Transfer or Approval events are consumed here.
        tables
            .upsert_row("token_holder_refresh_state", &basket)
            .set("dirty", true);
    }

    for event in basket_events.trades {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let basket = prefixed_hex(&event.basket);
        let timestamp = event_timestamp(event.evt_block_time);
        let usdg_amount = parse_bigint(&event.usdg_amount);
        let fee_weth = parse_bigint(&event.fee_weth);
        let basket_row = tables.upsert_row("baskets", &basket);
        if event.is_buy {
            basket_row.add("buy_count", 1);
        } else {
            basket_row.add("sell_count", 1);
        }
        basket_row
            .add("total_usdg_volume", &usdg_amount)
            .add("total_fee_weth", &fee_weth);

        let row = tables
            .upsert_row("basket_trade_events", &id)
            .set("basket", &basket)
            .set("is_buy", event.is_buy)
            .set("payer", prefixed_hex(&event.payer))
            .set("frontend", prefixed_hex(&event.frontend))
            .set("usdg_amount", usdg_amount)
            .set("basket_amount", event.basket_amount)
            .set("fee_weth", fee_weth)
            .set("routed", event.routed)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
        if !event.recipient.is_empty() {
            row.set("recipient", prefixed_hex(&event.recipient));
        }
        if event.routed {
            row.set("router_log_index", event.router_evt_index);
        }

        tables
            .upsert_row("token_holder_refresh_state", &basket)
            .set("dirty", true)
            .max("last_trade_entity_index", event.evt_ordinal as i64)
            .max("last_trade_block", event.evt_block_number)
            .max("last_trade_timestamp", timestamp);
    }

    for event in basket_events.operations {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let row = tables
            .upsert_row("basket_operations", &id)
            .set("operation_type", event.kind)
            .set("basket", prefixed_hex(&event.basket))
            .set("amount", event.amount)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", event_timestamp(event.evt_block_time))
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
        if !event.account.is_empty() {
            row.set("account", prefixed_hex(&event.account));
        }
        if !event.recipient.is_empty() {
            row.set("recipient", prefixed_hex(&event.recipient));
        }
        if !event.asset.is_empty() {
            row.set("asset", prefixed_hex(&event.asset));
        }
    }

    for event in basket_events.fee_accruals {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        tables
            .upsert_row("basket_fee_accrual_events", &id)
            .set("basket", prefixed_hex(&event.basket))
            .set("holder_amount", event.holder_amount)
            .set("auction_amount", event.auction_amount)
            .set("creator_amount", event.creator_amount)
            .set("launcher_amount", event.launcher_amount)
            .set("frontend", prefixed_hex(&event.frontend))
            .set("frontend_amount", event.frontend_amount)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", event_timestamp(event.evt_block_time))
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
    }

    for event in basket_events.fee_claims {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        tables
            .upsert_row("basket_fee_claim_events", &id)
            .set("basket", prefixed_hex(&event.basket))
            .set("claim_type", event.claim_type)
            .set("beneficiary", prefixed_hex(&event.beneficiary))
            .set("amount", event.amount)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", event_timestamp(event.evt_block_time))
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
    }

    for event in basket_events.auction_events {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let timestamp = event_timestamp(event.evt_block_time);
        match event.kind.as_str() {
            "AUCTION_CREATED" => {
                tables
                    .upsert_row("basket_auctions", &event.auction_id)
                    .set("creator", prefixed_hex(&event.account))
                    .set("eth_amount", &event.amount)
                    .set("spot_quote", event.spot_quote)
                    .set("initial_bid", &event.initial_bid)
                    .set("highest_bid", &event.initial_bid)
                    .set("highest_bidder", prefixed_hex(&event.account))
                    .set("start_time", timestamp)
                    .set("end_time", event.end_time)
                    .set("status", "ACTIVE")
                    .set("creation_transaction_hash", &event.evt_tx_hash)
                    .set("creation_log_index", event.evt_index);
                tables
                    .upsert_row("basket_auction_bid_events", &id)
                    .set("auction_id", event.auction_id)
                    .set("bidder", prefixed_hex(&event.account))
                    .set("total_bid", event.initial_bid)
                    .set("is_initial", true)
                    .set("block_number", event.evt_block_number)
                    .set("block_hash", prefixed_hex(&event.evt_block_hash))
                    .set("block_timestamp", timestamp)
                    .set("transaction_hash", event.evt_tx_hash)
                    .set("log_index", event.evt_index);
            }
            "BID_PLACED" => {
                tables
                    .upsert_row("basket_auctions", &event.auction_id)
                    .set("highest_bid", &event.amount)
                    .set("highest_bidder", prefixed_hex(&event.account));
                tables
                    .upsert_row("basket_auction_bid_events", &id)
                    .set("auction_id", event.auction_id)
                    .set("bidder", prefixed_hex(&event.account))
                    .set("total_bid", event.amount)
                    .set("is_initial", false)
                    .set("block_number", event.evt_block_number)
                    .set("block_hash", prefixed_hex(&event.evt_block_hash))
                    .set("block_timestamp", timestamp)
                    .set("transaction_hash", event.evt_tx_hash)
                    .set("log_index", event.evt_index);
            }
            "AUCTION_SETTLED" => {
                tables
                    .upsert_row("basket_auctions", &event.auction_id)
                    .set("highest_bid", &event.secondary_amount)
                    .set("highest_bidder", prefixed_hex(&event.account))
                    .set("settled_at", timestamp)
                    .set("status", "SETTLED");
                tables
                    .upsert_row("basket_auction_results", &event.auction_id)
                    .set("winner", prefixed_hex(&event.account))
                    .set("eth_amount", event.amount)
                    .set("bid_token_burned", event.secondary_amount)
                    .set("settled_at", timestamp)
                    .set("transaction_hash", event.evt_tx_hash)
                    .set("log_index", event.evt_index);
            }
            _ => {
                let row = tables
                    .upsert_row("basket_auction_account_events", &id)
                    .set("event_type", event.kind)
                    .set("account", prefixed_hex(&event.account))
                    .set("amount", event.amount)
                    .set("block_number", event.evt_block_number)
                    .set("block_hash", prefixed_hex(&event.evt_block_hash))
                    .set("block_timestamp", timestamp)
                    .set("transaction_hash", event.evt_tx_hash)
                    .set("log_index", event.evt_index);
                if !event.auction_id.is_empty() {
                    row.set("auction_id", event.auction_id);
                }
                if !event.recipient.is_empty() {
                    row.set("recipient", prefixed_hex(&event.recipient));
                }
            }
        }
    }

    for event in basket_events.rebalances {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        tables
            .upsert_row("basket_rebalances", &id)
            .set("basket", prefixed_hex(&event.basket))
            .set("nav_before", event.nav_before)
            .set("nav_after", event.nav_after)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", event_timestamp(event.evt_block_time))
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
    }
}

#[substreams::handlers::map]
fn basket_db_out(
    basket_registry_events: contract::BasketRegistryEvents,
    basket_events: contract::BasketEvents,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    let mut tables = Tables::new();
    write_basket_changes(&mut tables, basket_registry_events, basket_events);
    Ok(tables.to_database_changes())
}

#[substreams::handlers::map]
fn db_out(
    events: contract::Events,
    token_events: contract::TokenEvents,
    swap_events: contract::TokenEvents,
    ipshare_events: contract::IpShareEvents,
    walnut_factory_events: contract::WalnutEvents,
    walnut_events: contract::WalnutEvents,
    basket_registry_events: contract::BasketRegistryEvents,
    basket_events: contract::BasketEvents,
    bonding_curve_supply: StoreGetBigInt,
    entity_indexes: StoreGetInt64,
    ipshare_indexes: StoreGetInt64,
    ipshare_holder_deltas: Deltas<DeltaBigInt>,
    ipshare_stake_deltas: Deltas<DeltaBigInt>,
    walnut_indexes: StoreGetInt64,
    walnut_contracts: StoreGetString,
    walnut_owners: StoreGetString,
    walnut_membership_deltas: Deltas<DeltaInt64>,
    pre_walnut_account_deltas: Deltas<DeltaString>,
    token_account_deltas: Deltas<DeltaString>,
    walnut_account_deltas: Deltas<DeltaString>,
    user_account_indexes: StoreGetInt64,
    walnut_account_indexes: StoreGetInt64,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    let mut tables = Tables::new();
    write_basket_changes(&mut tables, basket_registry_events, basket_events);

    for event in events.pump_new_tokens {
        let token_id = format!("0x{}", Hex(&event.token));
        let creator = format!("0x{}", Hex(&event.creator));
        let symbol = event.tick.clone();
        let timestamp = event
            .evt_block_time
            .map(|value| value.seconds)
            .unwrap_or_default();

        tables
            .upsert_row("pump_token_discoveries", &token_id)
            .set("token", &token_id)
            .set("creator", &creator)
            .set("symbol", &symbol)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);

        tables
            .upsert_row("tokens", &token_id)
            .set(
                "entity_index",
                entity_indexes
                    .get_at(event.evt_ordinal, "token")
                    .expect("token index exists at NewToken ordinal"),
            )
            .set("symbol", symbol)
            .set("creator", creator)
            .set("pump", prefixed_hex(&PUMP_TRACKED_CONTRACT))
            .set("version", 9)
            .set("listed", false)
            .set("buy_times", 0)
            .set("sell_times", 0)
            .set("tiptag_fee", 0)
            .set("sellsman_fee", 0)
            .set("bonding_curve_supply", 0)
            .set("max_bonding_curve_supply", 0)
            .set("price", 0)
            .set("creation_block", event.evt_block_number)
            .set("creation_log_index", event.evt_index);
        tables
            .upsert_row("pump_summary", "pump")
            .add("token_counts", 1);

        // A newly discovered token must enter the four-hour fallback schedule
        // even when it has not emitted its first Trade yet.
        tables
            .upsert_row("token_holder_refresh_state", &token_id)
            .set("dirty", true);
    }

    for event in token_events.trades {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let token = prefixed_hex(&event.token);
        let timestamp = event_timestamp(event.evt_block_time);
        let trade_index = entity_indexes
            .get_at(event.evt_ordinal, "token_trade")
            .expect("trade index exists at Trade ordinal");
        let token_amount = BigInt::from_str(&event.token_amount).expect("valid token amount");
        let tiptag_fee = BigInt::from_str(&event.tiptag_fee).expect("valid tiptag fee");
        let sellsman_fee = BigInt::from_str(&event.sellsman_fee).expect("valid sellsman fee");
        let supply = bonding_curve_supply
            .get_at(event.evt_ordinal, &token)
            .expect("bonding curve supply exists at Trade ordinal");
        let price = bonding_curve_price(&supply);

        let token_row = tables.upsert_row("tokens", &token);
        if event.is_buy {
            token_row.add("buy_times", 1);
        } else {
            token_row.add("sell_times", 1);
        }
        token_row
            .add("tiptag_fee", &tiptag_fee)
            .add("sellsman_fee", &sellsman_fee)
            .set("bonding_curve_supply", &supply)
            .max("max_bonding_curve_supply", &supply)
            .set("price", &price);

        tables
            .upsert_row("token_trade_events", &id)
            .set("entity_index", trade_index)
            .set("token", token.as_str())
            .set("buyer", prefixed_hex(&event.buyer))
            .set("sellsman", prefixed_hex(&event.sellsman))
            .set("is_buy", event.is_buy)
            .set("token_amount", token_amount)
            .set("eth_amount", event.eth_amount)
            .set("tiptag_fee", tiptag_fee)
            .set("sellsman_fee", sellsman_fee)
            .set("price", price)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);

        tables
            .upsert_row("token_holder_refresh_state", &token)
            .set("dirty", true)
            .max("last_trade_entity_index", trade_index)
            .max("last_trade_block", event.evt_block_number)
            .max("last_trade_timestamp", timestamp);
    }

    for event in swap_events.swap_trades {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let token = prefixed_hex(&event.token);
        let timestamp = event_timestamp(event.evt_block_time);
        let trade_index = entity_indexes
            .get_at(event.evt_ordinal, "token_trade")
            .expect("swap trade index exists");
        let price = parse_bigint(&event.price);
        let tiptag_fee = parse_bigint(&event.tiptag_fee);
        let sellsman_fee = parse_bigint(&event.sellsman_fee);
        let token_row = tables.upsert_row("tokens", &token).set("price", &price);
        if event.is_buy {
            token_row.add("buy_times", 1);
        } else {
            token_row.add("sell_times", 1);
        }
        tables
            .upsert_row("token_trade_events", &id)
            .set("entity_index", trade_index)
            .set("token", token.as_str())
            .set("buyer", prefixed_hex(&event.buyer))
            .set("sellsman", prefixed_hex(&event.sellsman))
            .set("is_buy", event.is_buy)
            .set("token_amount", event.token_amount)
            .set("eth_amount", event.eth_amount)
            .set("tiptag_fee", tiptag_fee)
            .set("sellsman_fee", sellsman_fee)
            .set("price", price)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);

        tables
            .upsert_row("token_holder_refresh_state", &token)
            .set("dirty", true)
            .max("last_trade_entity_index", trade_index)
            .max("last_trade_block", event.evt_block_number)
            .max("last_trade_timestamp", timestamp);
    }

    for event in token_events.listed_to_dex {
        let token = prefixed_hex(&event.token);
        tables.upsert_row("tokens", &token).set("listed", true);

        tables
            .upsert_row("token_listings", &token)
            .set(
                "entity_index",
                entity_indexes
                    .get_at(event.evt_ordinal, "listed_token")
                    .expect("listing index exists at TokenListedToDex ordinal"),
            )
            .set("event_token", prefixed_hex(&event.event_token))
            .set("pool_id", prefixed_hex(&event.pool_id))
            .set("sqrt_price_x96", event.sqrt_price_x96)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", event_timestamp(event.evt_block_time))
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
        tables
            .upsert_row("pump_summary", "pump")
            .add("listed_counts", 1);
        tables
            .upsert_row("pairs", prefixed_hex(&event.pool_id))
            .set("token", &token)
            .set("token_index", 1);
    }

    for event in ipshare_events.creates {
        let subject = prefixed_hex(&event.subject);
        let amount = BigInt::from_str(&event.amount).expect("valid IPShare amount");
        let create_fee = BigInt::from_str(&event.create_fee).expect("valid create fee");
        let timestamp = event_timestamp(event.evt_block_time);
        ensure_account(&mut tables, &subject, timestamp)
            .set(
                "ipshare_index",
                ipshare_indexes
                    .get_at(event.evt_ordinal, "ipshare")
                    .expect("IPShare index exists at Create ordinal"),
            )
            .set("share_supply", &amount)
            .set("ipshare_create_block", event.evt_block_number);

        tables
            .upsert_row("ipshare_holders", holder_key(&subject, &subject))
            .set("holder", &subject)
            .set("subject", &subject)
            .set("shares_owned", amount)
            .set_if_null("created_at", timestamp);
        tables
            .upsert_row("ipshare_summary", "summary")
            .add("total_create_fee", create_fee)
            .add("buy_count", 1);
    }

    for event in ipshare_events.trades {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let trader = prefixed_hex(&event.trader);
        let subject = prefixed_hex(&event.subject);
        let amount = BigInt::from_str(&event.share_amount).expect("valid share amount");
        let protocol_fee =
            BigInt::from_str(&event.protocol_eth_amount).expect("valid protocol fee");
        let subject_fee = BigInt::from_str(&event.subject_eth_amount).expect("valid subject fee");
        let timestamp = event_timestamp(event.evt_block_time);
        ensure_account(&mut tables, &trader, timestamp);
        ensure_account(&mut tables, &subject, timestamp)
            .set("share_supply", &event.supply)
            .add("fee_amount", subject_fee);

        let holder = tables
            .upsert_row("ipshare_holders", holder_key(&trader, &subject))
            .set("holder", &trader)
            .set("subject", &subject)
            .set_if_null("created_at", timestamp);
        if event.is_buy {
            holder.add("shares_owned", &amount);
        } else {
            holder.sub("shares_owned", &amount);
        }

        let summary = tables
            .upsert_row("ipshare_summary", "summary")
            .add("total_protocol_fee", protocol_fee);
        if event.is_buy {
            summary.add("buy_count", 1);
        } else {
            summary.add("sell_count", 1);
        }

        tables
            .upsert_row("ipshare_trade_events", &id)
            .set(
                "entity_index",
                ipshare_indexes
                    .get_at(event.evt_ordinal, "ipshare_trade")
                    .expect("trade index exists"),
            )
            .set("trader", trader)
            .set("subject", subject)
            .set("is_buy", event.is_buy)
            .set("share_amount", amount)
            .set("eth_amount", event.eth_amount)
            .set("protocol_eth_amount", event.protocol_eth_amount)
            .set("subject_eth_amount", event.subject_eth_amount)
            .set("supply", event.supply)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
    }

    for event in ipshare_events.value_captures {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let subject = prefixed_hex(&event.subject);
        let investor = prefixed_hex(&event.investor);
        let amount = BigInt::from_str(&event.amount).expect("valid captured amount");
        let timestamp = event_timestamp(event.evt_block_time);
        ensure_account(&mut tables, &investor, timestamp);
        ensure_account(&mut tables, &subject, timestamp)
            .add("capture_count", 1)
            .add("total_captured", &amount);
        tables
            .upsert_row("ipshare_summary", "summary")
            .add("total_value_capture", &amount);
        tables
            .upsert_row("ipshare_value_capture_events", &id)
            .set(
                "entity_index",
                ipshare_indexes
                    .get_at(event.evt_ordinal, "value_capture")
                    .expect("capture index exists"),
            )
            .set("subject", subject)
            .set("investor", investor)
            .set("amount", amount)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
    }

    for event in ipshare_events.stakes {
        let id = event_id(&event.evt_tx_hash, event.evt_index);
        let staker = prefixed_hex(&event.staker);
        let subject = prefixed_hex(&event.subject);
        let amount = BigInt::from_str(&event.amount).expect("valid stake amount");
        let timestamp = event_timestamp(event.evt_block_time);
        ensure_account(&mut tables, &staker, timestamp);
        ensure_account(&mut tables, &subject, timestamp).set("total_staked", event.staked_amount);
        let row = tables
            .upsert_row("ipshare_stakers", holder_key(&staker, &subject))
            .set("staker", &staker)
            .set("subject", &subject)
            .set_if_null("created_at", timestamp);
        if event.is_stake {
            row.add("staked_amount", &amount);
        } else {
            row.sub("staked_amount", &amount);
        }
        tables
            .upsert_row("ipshare_stake_events", &id)
            .set(
                "entity_index",
                ipshare_indexes
                    .get_at(event.evt_ordinal, "stake")
                    .expect("stake index exists"),
            )
            .set("staker", staker)
            .set("subject", subject)
            .set("is_stake", event.is_stake)
            .set("share_amount", amount)
            .set("block_number", event.evt_block_number)
            .set("block_hash", prefixed_hex(&event.evt_block_hash))
            .set("block_timestamp", timestamp)
            .set("transaction_hash", event.evt_tx_hash)
            .set("log_index", event.evt_index);
    }

    for delta in ipshare_holder_deltas.deltas {
        let Some((holder, subject)) = parse_holder_key(&delta.key) else {
            continue;
        };
        tables
            .upsert_row("ipshare_holders", &delta.key)
            .set("holder", holder)
            .set("subject", subject)
            .set("shares_owned", &delta.new_value);
        apply_relation_count_delta(
            &mut tables,
            holder,
            "holdings_count",
            subject,
            "holders_count",
            &delta.old_value,
            &delta.new_value,
        );
    }

    for delta in ipshare_stake_deltas.deltas {
        let Some((staker, subject)) = parse_holder_key(&delta.key) else {
            continue;
        };
        tables
            .upsert_row("ipshare_stakers", &delta.key)
            .set("staker", staker)
            .set("subject", subject)
            .set("staked_amount", &delta.new_value);
        apply_relation_count_delta(
            &mut tables,
            staker,
            "staked_count",
            subject,
            "stakers_count",
            &delta.old_value,
            &delta.new_value,
        );
    }

    let factory_community_count = walnut_factory_events
        .events
        .iter()
        .filter(|event| event.kind == "COMMUNITY_CREATED")
        .count() as i64;
    let factory_pool_count = walnut_factory_events.events.len() as i64 - factory_community_count;
    let factory_operation_count = walnut_factory_events.events.len() as i64;
    let dynamic_operation_count = walnut_events
        .events
        .iter()
        .filter(|event| {
            !matches!(
                event.kind.as_str(),
                "POOL_UPDATED" | "OWNERSHIP_TRANSFERRED"
            )
        })
        .count() as i64;
    let mut next_community_index =
        walnut_indexes.get_last("walnut_community").unwrap_or(0) - factory_community_count + 1;
    let mut next_pool_index =
        walnut_indexes.get_last("walnut_pool").unwrap_or(0) - factory_pool_count + 1;
    let mut next_operation_index = walnut_indexes.get_last("walnut_op").unwrap_or(0)
        - factory_operation_count
        - dynamic_operation_count
        + 1;

    for event in walnut_factory_events.events {
        let timestamp = event_timestamp(event.evt_block_time.clone());
        if event.kind == "COMMUNITY_CREATED" {
            let community = prefixed_hex(&event.community);
            let creator = prefixed_hex(&event.account);
            ensure_account(&mut tables, &creator, timestamp);
            tables
                .upsert_row("walnut_communities", &community)
                .set("entity_index", next_community_index)
                .set("created_at", timestamp)
                .set("owner", &creator)
                .set("dao_fund", &creator)
                .set("treasury", &creator)
                .set("c_token", prefixed_hex(&event.pool));
            next_community_index += 1;
            tables
                .upsert_row("walnut_summary", "walnut")
                .add("total_communities", 1);
            write_walnut_operation(
                &mut tables,
                &event,
                next_operation_index,
                "ADMINCREATE",
                &creator,
                None,
                None,
                None,
            );
            next_operation_index += 1;
        } else {
            let pool = prefixed_hex(&event.pool);
            let community = prefixed_hex(&event.community);
            let descriptor = walnut_contracts.get_at(event.evt_ordinal, &community);
            let community_token = descriptor
                .as_deref()
                .and_then(|v| v.split('|').nth(1))
                .unwrap_or("0x");
            let (pool_type, asset, lock_duration) = match event.kind.as_str() {
                "ERC20_STAKING_CREATED" => ("ERC20_STAKING", prefixed_hex(&event.asset), None),
                "ERC20_LOCKING_CREATED" => (
                    "ERC20_LOCKING",
                    prefixed_hex(&event.asset),
                    Some(event.amount.clone()),
                ),
                _ => ("SOCIAL_CURATION", community_token.to_string(), None),
            };
            tables
                .upsert_row("walnut_pools", &pool)
                .set("entity_index", next_pool_index)
                .set("created_at", timestamp)
                .set("status", "OPENED")
                .set("name", &event.name)
                .set("pool_factory", prefixed_hex(&event.contract))
                .set("community", &community)
                .set("asset", asset)
                .set("tvl", 0)
                .set("pool_type", pool_type);
            next_pool_index += 1;
            if let Some(duration) = lock_duration {
                tables
                    .upsert_row("walnut_pools", &pool)
                    .set("lock_duration", duration);
            }
            tables
                .upsert_row("walnut_communities", &community)
                .add("pools_count", 1);
            tables
                .upsert_row("walnut_summary", "walnut")
                .add("total_pools", 1);
            let owner = walnut_owners
                .get_at(event.evt_ordinal, &community)
                .unwrap_or_else(|| prefixed_hex(&event.operator));
            ensure_account(&mut tables, &owner, timestamp);
            let pool_factory = prefixed_hex(&event.contract);
            write_walnut_operation(
                &mut tables,
                &event,
                next_operation_index,
                "ADMINADDPOOL",
                &owner,
                Some(&pool),
                Some(&pool_factory),
                None,
            );
            next_operation_index += 1;
        }
    }

    for event in walnut_events.events {
        let community = prefixed_hex(&event.community);
        let pool = prefixed_hex(&event.pool);
        let amount = if event.amount.is_empty() {
            BigInt::from(0)
        } else {
            parse_bigint(&event.amount)
        };
        let mut create_op = true;
        match event.kind.as_str() {
            "ADMINSETFEE" => {
                tables
                    .upsert_row("walnut_communities", &community)
                    .set("fee_ratio", event.ratio);
            }
            "ADMINSETRATIO" => {
                tables
                    .upsert_row("walnut_communities", &community)
                    .set("active_pool_count", event.pools.len() as i64);
                for (idx, address) in event.pools.iter().enumerate() {
                    let row = tables
                        .upsert_row("walnut_pools", prefixed_hex(address))
                        .set("pool_index", idx as i64);
                    if let Some(ratio) = event.ratios.get(idx) {
                        row.set("ratio", *ratio);
                    }
                }
            }
            "POOL_UPDATED" => {
                tables
                    .upsert_row("walnut_communities", &community)
                    .add("revenue", &amount)
                    .add("retained_revenue", &amount);
                create_op = false;
            }
            "ADMINCLOSEPOOL" => {
                tables
                    .upsert_row("walnut_pools", &pool)
                    .set("status", "CLOSED");
            }
            "OWNERSHIP_TRANSFERRED" => {
                if !is_zero_address(&event.secondary_account) {
                    tables
                        .upsert_row("walnut_communities", &community)
                        .set("owner", prefixed_hex(&event.account));
                }
                create_op = false;
            }
            "ADMINSETDAOFUND" => {
                tables
                    .upsert_row("walnut_communities", &community)
                    .set("dao_fund", prefixed_hex(&event.account));
            }
            "ADMINWITHDRAWNREVENUE" => {
                tables
                    .upsert_row("walnut_communities", &community)
                    .sub("retained_revenue", &amount);
            }
            "HARVEST" | "HARVESTALL" => {
                tables
                    .upsert_row("walnut_communities", &community)
                    .add("distributed_c_token", &amount);
            }
            "DEPOSIT" | "LOCK" => {
                tables
                    .upsert_row("walnut_pools", &pool)
                    .add("total_amount", &amount);
            }
            "WITHDRAW" | "UNLOCK" => {
                tables
                    .upsert_row("walnut_pools", &pool)
                    .sub("total_amount", &amount);
            }
            "REDEEM" | "SOCIALCLAIM" => {}
            _ => {
                create_op = false;
            }
        }
        if create_op {
            let is_admin = event.kind.starts_with("ADMIN");
            let account = if is_admin {
                walnut_owners
                    .get_at(event.evt_ordinal, &community)
                    .unwrap_or_else(|| prefixed_hex(&event.operator))
            } else if event.account.is_empty() {
                prefixed_hex(&event.operator)
            } else {
                prefixed_hex(&event.account)
            };
            ensure_account(
                &mut tables,
                &account,
                event_timestamp(event.evt_block_time.clone()),
            );
            let asset = if matches!(
                event.kind.as_str(),
                "ADMINSETDAOFUND" | "ADMINWITHDRAWNREVENUE"
            ) {
                Some(prefixed_hex(&event.account))
            } else if event.asset.is_empty() {
                None
            } else {
                Some(prefixed_hex(&event.asset))
            };
            let pool_factory = if event.pool_factory.is_empty() {
                None
            } else {
                Some(prefixed_hex(&event.pool_factory))
            };
            write_walnut_operation(
                &mut tables,
                &event,
                next_operation_index,
                &event.kind,
                &account,
                if event.pool.is_empty() {
                    None
                } else {
                    Some(&pool)
                },
                pool_factory.as_deref(),
                asset.as_deref(),
            );
            // Community.createAdminOp always materializes chainId=0 for
            // dynamic admin handlers. Factory ADMINCREATE/ADMINADDPOOL rows
            // intentionally keep it NULL, matching the legacy mappings.
            if is_admin {
                tables
                    .upsert_row(
                        "walnut_operations",
                        event_id(&event.evt_tx_hash, event.evt_index),
                    )
                    .set("chain_id", 0);
            }
            next_operation_index += 1;
        }
    }

    for delta in walnut_membership_deltas.deltas {
        let parts: Vec<&str> = delta.key.split('|').collect();
        match parts.as_slice() {
            ["community", community, account] => {
                tables
                    .upsert_row("walnut_account_communities", &delta.key)
                    .set("account", *account)
                    .set("community", *community)
                    .set("created_at", delta.new_value);
                tables
                    .upsert_row("walnut_communities", *community)
                    .add("users_count", 1);
            }
            ["pool", pool, account] => {
                tables
                    .upsert_row("walnut_account_pools", &delta.key)
                    .set("account", *account)
                    .set("pool", *pool)
                    .set("created_at", delta.new_value);
            }
            ["staker", pool, account] => {
                tables
                    .upsert_row("walnut_pool_stakers", &delta.key)
                    .set("account", *account)
                    .set("pool", *pool)
                    .set("created_at", delta.new_value);
                tables
                    .upsert_row("walnut_pools", *pool)
                    .add("stakers_count", 1);
            }
            _ => {}
        }
    }

    let mut pre_walnut_account_deltas = pre_walnut_account_deltas.deltas;
    pre_walnut_account_deltas.sort_by_key(|delta| delta.ordinal);
    let mut token_account_deltas = token_account_deltas.deltas;
    token_account_deltas.sort_by_key(|delta| delta.ordinal);
    let user_count = pre_walnut_account_deltas.len() + token_account_deltas.len();
    let mut next_user_index =
        user_account_indexes.get_last("user").unwrap_or(0) - user_count as i64 + 1;
    for delta in pre_walnut_account_deltas
        .into_iter()
        .chain(token_account_deltas)
    {
        let timestamp = delta.new_value.parse::<i64>().unwrap_or_default();
        tables
            .upsert_row("ipshare_summary", "summary")
            .add("users_count", 1);
        tables
            .upsert_row("accounts", &delta.key)
            .set_if_null("joined_at", timestamp)
            .set("entity_index", next_user_index);
        next_user_index += 1;
    }

    let mut walnut_account_deltas = walnut_account_deltas.deltas;
    walnut_account_deltas.sort_by_key(|delta| {
        let mut parts = delta.new_value.split('|');
        let _timestamp = parts.next();
        let priority = parts
            .next()
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(9);
        (priority, delta.ordinal)
    });
    let mut next_walnut_user_index = walnut_account_indexes.get_last("walnut_user").unwrap_or(0)
        - walnut_account_deltas.len() as i64
        + 1;
    for delta in walnut_account_deltas {
        let mut value = delta.new_value.split('|');
        let timestamp = value
            .next()
            .and_then(|part| part.parse::<i64>().ok())
            .unwrap_or_default();
        tables
            .upsert_row("walnut_summary", "walnut")
            .add("total_users", 1);
        tables
            .upsert_row("accounts", &delta.key)
            .set_if_null("joined_at", timestamp)
            .set("entity_index", next_walnut_user_index);
        next_walnut_user_index += 1;
    }

    Ok(tables.to_database_changes())
}

fn token_key(address: &[u8]) -> String {
    prefixed_hex(address)
}

fn prefixed_hex(bytes: &[u8]) -> String {
    format!("0x{}", Hex(bytes))
}

fn event_id(transaction_hash: &str, log_index: u32) -> String {
    format!("{}-{}", transaction_hash, log_index)
}

fn event_timestamp(timestamp: Option<prost_types::Timestamp>) -> i64 {
    timestamp.map(|value| value.seconds).unwrap_or_default()
}

fn is_zero_address(address: &[u8]) -> bool {
    address.iter().all(|byte| *byte == 0)
}

fn holder_key(holder: &str, subject: &str) -> String {
    format!("{}:{}", holder, subject)
}

fn parse_holder_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(':')
}

fn parse_bigint(value: &str) -> BigInt {
    BigInt::from_str(value).expect("valid uint256 value")
}

fn apply_relation_count_delta(
    tables: &mut Tables,
    owner: &str,
    owner_field: &str,
    subject: &str,
    subject_field: &str,
    old_value: &BigInt,
    new_value: &BigInt,
) {
    let zero = BigInt::from(0);
    let change = if old_value == &zero && new_value > &zero {
        1
    } else if old_value > &zero && new_value == &zero {
        -1
    } else {
        0
    };
    if change != 0 {
        tables
            .upsert_row("accounts", owner)
            .add(owner_field, change);
        tables
            .upsert_row("accounts", subject)
            .add(subject_field, change);
    }
}

fn ensure_account<'a>(
    tables: &'a mut Tables,
    address: &str,
    timestamp: i64,
) -> &'a mut substreams_database_change::tables::Row {
    tables
        .upsert_row("accounts", address)
        .set_if_null("joined_at", timestamp)
}

fn write_walnut_operation(
    tables: &mut Tables,
    event: &contract::WalnutEvent,
    index: i64,
    operation_type: &str,
    account: &str,
    pool: Option<&str>,
    pool_factory: Option<&str>,
    asset: Option<&str>,
) {
    let id = event_id(&event.evt_tx_hash, event.evt_index);
    let community = prefixed_hex(&event.community);
    let row = tables
        .upsert_row("walnut_operations", &id)
        .set("entity_index", index)
        .set("operation_type", operation_type)
        .set("community", &community)
        .set("account", account)
        .set("block_number", event.evt_block_number)
        .set("block_hash", prefixed_hex(&event.evt_block_hash))
        .set(
            "block_timestamp",
            event_timestamp(event.evt_block_time.clone()),
        )
        .set("transaction_hash", &event.evt_tx_hash)
        .set("log_index", event.evt_index);
    if let Some(pool) = pool {
        row.set("pool", pool);
    }
    if let Some(pool_factory) = pool_factory {
        row.set("pool_factory", pool_factory);
    }
    if let Some(asset) = asset {
        row.set("asset", asset);
    }
    if !event.amount.is_empty() {
        row.set("amount", &event.amount);
    }
    if !event.secondary_amount.is_empty() {
        row.set("social_order_id", &event.secondary_amount);
    }
    if event.kind == "SOCIALCLAIM" {
        row.set("social_harvested", event.flag);
    }
    tables
        .upsert_row("walnut_communities", &community)
        .add("operation_count", 1);
    tables
        .upsert_row("accounts", account)
        .add("walnut_operation_count", 1);
}

fn bonding_curve_price(supply: &BigInt) -> BigInt {
    const INIT_PRICE: u64 = 6_500_000_000;
    const SCALE: u64 = 100_000_000;
    const CURVE_DENOMINATOR: f64 = 251_755_164_380_000_000_000_000_000.0;

    let supply = supply
        .to_string()
        .parse::<f64>()
        .expect("bonding curve supply fits f64");
    let scaled_exp = ((supply / CURVE_DENOMINATOR).exp() * SCALE as f64) as u64;

    (BigInt::from(INIT_PRICE) * BigInt::from(scaled_exp)) / BigInt::from(SCALE)
}

fn swap_price(sqrt_price_x96: &BigInt) -> BigInt {
    if sqrt_price_x96 == &BigInt::from(0) {
        return BigInt::from(0);
    }
    let numerator = (BigInt::from(1) << 192) * BigInt::from(1_000_000_000_000_000_000u64);
    numerator / (sqrt_price_x96 * sqrt_price_x96)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_rh_trade_price_matches_legacy_graph() {
        let supply = BigInt::from_str("99999999999999974150783").unwrap();
        assert_eq!(bonding_curve_price(&supply).to_string(), "6502582385");
    }

    #[test]
    fn zero_supply_uses_initial_price() {
        assert_eq!(
            bonding_curve_price(&BigInt::from(0)).to_string(),
            "6500000000"
        );
    }

    #[test]
    fn basket_router_events_are_paired_once_in_log_order() {
        let basket = vec![1; 20];
        let mut candidates = vec![
            BasketRouterTrade {
                basket: basket.clone(),
                payer: vec![2; 20],
                recipient: vec![3; 20],
                usdg_amount: "100".into(),
                basket_amount: "90".into(),
                is_buy: true,
                log_index: 12,
                matched: false,
            },
            BasketRouterTrade {
                basket: basket.clone(),
                payer: vec![4; 20],
                recipient: vec![5; 20],
                usdg_amount: "100".into(),
                basket_amount: "90".into(),
                is_buy: true,
                log_index: 20,
                matched: false,
            },
        ];

        let first = take_router_trade(&mut candidates, 10, &basket, true, "100", "90").unwrap();
        let second = take_router_trade(&mut candidates, 18, &basket, true, "100", "90").unwrap();
        assert_eq!(first.2, 12);
        assert_eq!(second.2, 20);
        assert!(take_router_trade(&mut candidates, 1, &basket, true, "100", "90").is_none());
    }
}
