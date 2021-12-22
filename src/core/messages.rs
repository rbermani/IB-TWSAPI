//! Functions for processing messages
use std::any::Any;
use std::collections::HashSet;
use std::convert::TryInto;
use std::fmt::Formatter;
use std::io::Write;
use std::string::String;
use std::vec::Vec;

use ascii;
use ascii::AsAsciiStr;
use rust_decimal::Decimal;

use log::*;
use num_derive::FromPrimitive;

use crate::core::common::{
    BarData, CommissionReport, DepthMktDataDescription, FaDataType, FamilyCode, HistogramData,
    HistoricalTick, HistoricalTickBidAsk, HistoricalTickLast, NewsProvider, PriceIncrement,
    RealTimeBar, SmartComponent, TagValue, TickAttrib, TickMsgType, TickType, UNSET_DOUBLE, UNSET_INTEGER,
};
use crate::core::contract::{
    ComboLeg, ComboLegPreamble, Contract, ContractDescription, ContractDetails, ContractPreamble,
    DeltaNeutralContract,
};
use crate::core::errors::IBKRApiLibError;
use crate::core::execution::{Execution, ExecutionFilter};
use crate::core::order::{
    AuctionStrategy, Order, OrderComboLeg, OrderState, PlaceOrderPreamble, SoftDollarTier,
    VolatilityOrder,
};
use crate::core::order_condition::OrderConditionEnum;
use crate::core::scanner::ScannerSubscription;
use serde::de::{self, Deserializer, SeqAccess, Visitor};

use serde::{Deserialize, Serialize};

use strum_macros::Display;

//==================================================================================================
trait EClientMsgSink {
    fn server_version(version: i32, time: &str);
    fn redirect(host: &str);
}

//==================================================================================================
/// FA msg data types
pub enum FAMessageDataTypes {
    Groups = 1,
    Profiles = 2,
    Aliases = 3,
}

#[derive(FromPrimitive)]
#[repr(i32)]
pub enum ServerRspMsgDiscriminants {
    TickPrice = 1,
    TickSize = 2,
    OrderStatus = 3,
    ErrMsg = 4,
    OpenOrder = 5,
    AcctValue = 6,
    PortfolioValue = 7,
    AcctUpdateTime = 8,
    NextValidId = 9,
    ContractData = 10,
    ExecutionData = 11,
    MarketDepth = 12,
    MarketDepthL2 = 13,
    NewsBulletins = 14,
    ManagedAccts = 15,
    ReceiveFa = 16,
    HistoricalData = 17,
    BondContractData = 18,
    ScannerParameters = 19,
    ScannerData = 20,
    TickOptionComputation = 21,
    TickGeneric = 45,
    TickString = 46,
    TickEfp = 47,
    CurrentTime = 49,
    RealTimeBars = 50,
    FundamentalData = 51,
    ContractDataEnd = 52,
    OpenOrderEnd = 53,
    AcctDownloadEnd = 54,
    ExecutionDataEnd = 55,
    DeltaNeutralValidation = 56,
    TickSnapshotEnd = 57,
    MarketDataType = 58,
    CommissionReport = 59,
    PositionData = 61,
    PositionEnd = 62,
    AccountSummary = 63,
    AccountSummaryEnd = 64,
    VerifyMessageApi = 65,
    VerifyCompleted = 66,
    DisplayGroupList = 67,
    DisplayGroupUpdated = 68,
    VerifyAndAuthMessageApi = 69,
    VerifyAndAuthCompleted = 70,
    PositionMulti = 71,
    PositionMultiEnd = 72,
    AccountUpdateMulti = 73,
    AccountUpdateMultiEnd = 74,
    SecurityDefinitionOptionParameter = 75,
    SecurityDefinitionOptionParameterEnd = 76,
    SoftDollarTiers = 77,
    FamilyCodes = 78,
    SymbolSamples = 79,
    MktDepthExchanges = 80,
    TickReqParams = 81,
    SmartComponents = 82,
    NewsArticle = 83,
    TickNews = 84,
    NewsProviders = 85,
    HistoricalNews = 86,
    HistoricalNewsEnd = 87,
    HeadTimestamp = 88,
    HistogramData = 89,
    HistoricalDataUpdate = 90,
    RerouteMktDataReq = 91,
    RerouteMktDepthReq = 92,
    MarketRule = 93,
    Pnl = 94,
    PnlSingle = 95,
    HistoricalTicks = 96,
    HistoricalTicksBidAsk = 97,
    HistoricalTicksLast = 98,
    TickByTick = 99,
    OrderBound = 100,
    CompletedOrder = 101,
    CompletedOrdersEnd = 102,
}

#[derive(Clone, Serialize, Deserialize, Debug, Display)]
pub enum ServerRspMsg {
    TickPrice {
        version: i32,
        req_id: i32,
        tick_type: TickType,
        price: f64,
        tick_attr: TickAttrib,
    },
    TickSize {
        version: i32,
        req_id: i32,
        tick_type: TickType,
        size: i32,
    },
    OrderStatus {
        order_id: i32,
        status: String,
        filled: f64,
        remaining: f64,
        avg_fill_price: f64,
        perm_id: i32,
        parent_id: i32,
        last_fill_price: f64,
        client_id: i32,
        why_held: String,
        mkt_cap_price: f64,
    },
    ErrMsg {
        version: i32,
        req_id: i32,
        error_code: i32,
        error_str: String,
    },
    OpenOrder {
        order_id: i32,
        contract: Contract,
        order: Order,
        order_state: OrderState,
    },
    AcctValue {
        version: i32,
        key: String,
        val: String,
        currency: String,
        account_name: String,
    },
    PortfolioValue {
        version: i32,
        contract: Contract,
        position: f64,
        market_price: f64,
        market_value: f64,
        average_cost: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
        account_name: String,
    },
    AcctUpdateTime {
        version: i32,
        time_stamp: String,
    },
    NextValidId {
        version: i32,
        order_id: i32,
    },
    ContractData {
        version: i32,
        req_id: i32,
        contract_details: ContractDetails,
    },
    ExecutionData {
        req_id: i32,
        contract: Contract,
        execution: Execution,
    },
    MarketDepth {
        version: i32,
        req_id: i32,
        position: i32,
        operation: i32,
        side: i32,
        price: f64,
        size: i32,
    },
    MarketDepthL2 {
        version: i32,
        req_id: i32,
        position: i32,
        market_maker: String,
        operation: i32,
        side: i32,
        price: f64,
        size: i32,
        is_smart_depth: bool,
    },
    NewsBulletins {
        version: i32,
        msg_id: i32,
        msg_type: i32,
        news_message: String,
        origin_exch: String,
    },
    ManagedAccts {
        version: i32,
        accounts_list: String,
    },
    ReceiveFa {
        version: i32,
        fa_data: FaDataType,
        cxml: String,
    },
    HistoricalData {
        req_id: i32,
        bar: BarData,
    },
    BondContractData {
        version: i32,
        req_id: i32,
        contract_details: ContractDetails,
    },
    ScannerParameters {
        version: i32,
        xml: String,
    },
    ScannerData {
        version: i32,
        req_id: i32,
        rank: i32,
        contract_details: ContractDetails,
        distance: String,
        benchmark: String,
        projection: String,
        legs_str: String,
    },
    TickOptionComputation {
        version: i32,
        ticker_id: i32,
        tick_type: TickType,
        implied_vol: f64,
        delta: f64,
        opt_price: f64,
        pv_dividend: f64,
        gamma: f64,
        vega: f64,
        theta: f64,
        und_price: f64,
    },
    TickGeneric {
        version: i32,
        ticker_id: i32,
        tick_type: TickType,
        value: f64,
    },
    TickString {
        version: i32,
        req_id: i32,
        tick_type: TickType,
        value: String,
    },
    TickEfp {
        version: i32,
        ticker_id: i32,
        tick_type: TickType,
        basis_points: f64,
        formatted_basis_points: String,
        implied_futures_price: f64,
        hold_days: i32,
        future_last_trade_date: String,
        dividend_impact: f64,
        dividends_to_last_trade_date: f64,
    },
    CurrentTime {
        version: i32,
        time: i64,
    },
    RealTimeBars {
        version: i32,
        req_id: i32,
        bar: RealTimeBar,
    },
    FundamentalData {
        version: i32,
        req_id: i32,
        data: String,
    },
    ContractDataEnd {
        version: i32,
        req_id: i32,
    },
    OpenOrderEnd,
    AcctDownloadEnd {
        version: i32,
        account_name: String,
    },
    ExecutionDataEnd {
        version: i32,
        req_id: i32,
    },
    DeltaNeutralValidation {
        version: i32,
        req_id: i32,
        delta_neutral_contract: DeltaNeutralContract,
    },
    ScannerDataEnd {
        req_id: i32,
    },
    TickSnapshotEnd {
        version: i32,
        req_id: i32,
    },
    MarketDataType {
        version: i32,
        req_id: i32,
        market_data_type: i32,
    },
    CommissionReport {
        version: i32,
        commission_report: CommissionReport,
    },
    PositionData {
        version: i32,
        account: String,
        contract: Contract,
        position: f64,
        avg_cost: f64,
    },
    PositionEnd,
    AccountSummary {
        version: i32,
        req_id: i32,
        account: String,
        tag: String,
        value: String,
        currency: String,
    },
    AccountSummaryEnd {
        version: i32,
        req_id: i32,
    },
    VerifyMessageApi {
        version: i32,
        api_data: String,
    },
    VerifyCompleted {
        version: i32,
        is_successful: bool,
        error_text: String,
    },
    DisplayGroupList {
        version: i32,
        req_id: i32,
        groups: String,
    },
    DisplayGroupUpdated {
        version: i32,
        req_id: i32,
        contract_info: String,
    },
    VerifyAndAuthMessageApi {
        version: i32,
        api_data: String,
        xyz_challenge: String,
    },
    VerifyAndAuthCompleted {
        version: i32,
        is_successful: bool,
        error_text: String,
    },
    PositionMulti {
        version: i32,
        req_id: i32,
        account: String,
        model_code: String,
        contract: Contract,
        position: f64,
        avg_cost: f64,
    },
    PositionMultiEnd {
        version: i32,
        req_id: i32,
    },
    AccountUpdateMulti {
        version: i32,
        req_id: i32,
        account: String,
        model_code: String,
        key: String,
        value: String,
        currency: String,
    },
    AccountUpdateMultiEnd {
        version: i32,
        req_id: i32,
    },
    SecurityDefinitionOptionParameter {
        req_id: i32,
        exchange: String,
        underlying_con_id: i32,
        trading_class: String,
        multiplier: String,
        expirations: HashSet<String>,
        strikes: HashSet<Decimal>,
    },
    SecurityDefinitionOptionParameterEnd {
        req_id: i32,
    },
    SoftDollarTiers {
        req_id: i32,
        tiers: Vec<SoftDollarTier>,
    },
    FamilyCodes {
        family_codes: Vec<FamilyCode>,
    },
    SymbolSamples {
        req_id: i32,
        contract_descriptions: Vec<ContractDescription>,
    },
    MktDepthExchanges {
        depth_mkt_data_descriptions: Vec<DepthMktDataDescription>,
    },
    TickReqParams {
        ticker_id: i32,
        min_tick: f64,
        bbo_exchange: String,
        snapshot_permissions: i32,
    },
    SmartComponents {
        req_id: i32,
        smart_components: Vec<SmartComponent>,
    },
    NewsArticle {
        req_id: i32,
        article_type: i32,
        article_text: String,
    },
    TickNews {
        ticker_id: i32,
        time_stamp: i32,
        provider_code: String,
        article_id: String,
        headline: String,
        extra_data: String,
    },
    NewsProviders {
        news_providers: Vec<NewsProvider>,
    },
    HistoricalNews {
        req_id: i32,
        time: String,
        provider_code: String,
        article_id: String,
        headline: String,
    },
    HistoricalNewsEnd {
        req_id: i32,
        has_more: bool,
    },
    HeadTimestamp {
        req_id: i32,
        head_timestamp: String,
    },
    HistogramData {
        req_id: i32,
        items: Vec<HistogramData>,
    },
    HistoricalDataUpdate {
        req_id: i32,
        bar: BarData,
    },
    RerouteMktDataReq {
        req_id: i32,
        con_id: i32,
        exchange: String,
    },
    RerouteMktDepthReq {
        req_id: i32,
        con_id: i32,
        exchange: String,
    },
    MarketRule {
        market_rule_id: i32,
        price_increments: Vec<PriceIncrement>,
    },
    Pnl {
        req_id: i32,
        daily_pnl: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
    },
    PnlSingle {
        req_id: i32,
        pos: i32,
        daily_pnl: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
        value: f64,
    },
    HistoricalTicks {
        req_id: i32,
        ticks: Vec<HistoricalTick>,
        done: bool,
    },
    HistoricalTicksBidAsk {
        req_id: i32,
        ticks: Vec<HistoricalTickBidAsk>,
        done: bool,
    },
    HistoricalTicksLast {
        req_id: i32,
        ticks: Vec<HistoricalTickLast>,
        done: bool,
    },
    TickByTick {
        req_id: i32,
        tick_type: i32,
        time: i64,
        tick_msg: TickMsgType,
    },
    OrderBound {
        req_id: i32,
        api_client_id: i32,
        api_order_id: i32,
    },
    CompletedOrder {
        contract: Contract,
        order: Order,
        order_state: OrderState,
    },
    CompletedOrdersEnd,
    HistoricalDataEnd {
        req_id: i32,
        start: String,
        end: String,
    },
}

#[derive(Clone, Debug)]
pub struct ReqMktDataFields {
    contract: ContractPreamble,
    trading_class: String,
    combo_legs: Vec<ComboLegPreamble>,
    delta_neutral_contract: Option<DeltaNeutralContract>,
    generic_tick_list: String,
    snapshot: bool,
    regulatory_snapshot: bool,
    mkt_data_options: String, // internal use only, serialize as empty string field
}

impl<'de> serde::de::Deserialize<'de> for ReqMktDataFields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ReqMktDataFieldsVisitor;

        impl<'de> Visitor<'de> for ReqMktDataFieldsVisitor {
            type Value = ReqMktDataFields;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct ReqMktDataFields")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<ReqMktDataFields, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut combo_legs = vec![];
                let contract: ContractPreamble = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let trading_class = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                if contract.sec_type == "BAG" {
                    combo_legs = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                }

                let delta_neutral_contract: Option<DeltaNeutralContract> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;

                let generic_tick_list = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(5, &self))?;

                let snapshot = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(6, &self))?;

                let regulatory_snapshot = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(7, &self))?;
                let mkt_data_options = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(8, &self))?;

                Ok(ReqMktDataFields {
                    contract,
                    trading_class,
                    combo_legs,
                    delta_neutral_contract,
                    generic_tick_list,
                    snapshot,
                    regulatory_snapshot,
                    mkt_data_options,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "contract",
            "trading_class",
            "combo_legs",
            "delta_neutral_contract",
            "generic_tick_list",
            "snapshot",
            "regulatory_snapshot",
            "mkt_data_options",
        ];
        deserializer.deserialize_struct("ReqMktDataFields", FIELDS, ReqMktDataFieldsVisitor)
    }
}

#[derive(Clone, Debug)]
pub struct PlaceOrderFields {
    contract: ContractPreamble,
    trading_class: String,
    sec_id_type: String,
    sec_id: String,
    ord_hdr: PlaceOrderPreamble,
    contract_combo_legs: Vec<ComboLeg>,
    order_combo_legs: Vec<OrderComboLeg>,
    smart_combo_routing_params: Vec<TagValue>,
    shares_alloc_deprecated: i32, // deprecated field, empty string
    discretionary_amt: f64,
    good_after_time: String,
    good_till_date: String,
    fa_group: String,
    fa_method: String,
    fa_percentage: String,
    fa_profile: String,
    model_code: String,
    short_sale_slot: i32,
    designated_location: String,
    exempt_code: i32,
    oca_type: i32,
    rule80a: String,
    settling_firm: String,
    all_or_none: bool,
    min_qty: i32,
    percent_offset: f64,
    e_trade_only: bool,
    firm_quote_only: bool,
    nbbo_price_cap: f64,
    auction_strategy: AuctionStrategy,
    starting_price: f64,
    stock_ref_price: f64,
    delta: f64, // type: float
    stock_range_lower: f64,
    stock_range_upper: f64, // type: float
    override_percentage_constraints: bool,
    volat: VolatilityOrder,
    continuous_update: bool,
    reference_price_type: i32, // type: int; 1=Average, 2 = BidOrAsk
    trail_stop_price: f64,
    trailing_percent: f64, // type: float; TRAILLIMIT orders only
    scale_init_level_size: i32,
    scale_subs_level_size: i32,
    scale_price_increment: f64,
    scale_price_adjust_value: f64,
    scale_price_adjust_interval: i32,
    scale_profit_offset: f64,
    scale_auto_reset: bool,
    scale_init_position: i32,
    scale_init_fill_qty: i32,
    scale_random_percent: bool,
    scale_table: String,
    active_start_time: String,
    active_stop_time: String,

    hedge_type: String,
    hedge_param: String, // 'beta=X' value for beta hedge, 'ratio=Y' for pair hedge

    opt_out_smart_routing: bool,
    clearing_account: String,
    clearing_intent: String, // "" (Default), "IB", "Away", "PTA" (PostTrade)
    not_held: bool,
    delta_neutral_contract: Option<DeltaNeutralContract>,
    algo_strategy: String,
    algo_params: Vec<TagValue>,
    algo_id: String,
    what_if: bool,
    misc_options: String,
    solicited: bool,
    randomize_size: bool,
    randomize_price: bool,
    reference_contract_id: i32,
    is_pegged_change_amount_decrease: bool,
    pegged_change_amount: f64,
    reference_change_amount: f64,
    reference_exchange_id: String,
    conditions: Vec<OrderConditionEnum>,
    conditions_ignore_rth: bool,
    conditions_cancel_order: bool,
    adjusted_order_type: String,
    trigger_price: f64,
    lmt_price_offset: f64,
    adjusted_stop_price: f64,
    adjusted_stop_limit_price: f64,
    adjusted_trailing_amount: f64,
    adjustable_trailing_unit: i32,
    ext_operator: String,
    soft_dollar_tier: SoftDollarTier,
    cash_qty: f64,
    mifid2decision_maker: String,
    mifid2decision_algo: String,
    mifid2execution_trader: String,
    mifid2execution_algo: String,
    dont_use_auto_price_for_hedge: bool,
    is_oms_container: bool,
    discretionary_up_to_limit_price: bool,
    use_price_mgmt_algo: bool,
}

impl<'de> serde::de::Deserialize<'de> for PlaceOrderFields {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PlaceOrderFieldsVisitor;

        impl<'de> Visitor<'de> for PlaceOrderFieldsVisitor {
            type Value = PlaceOrderFields;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct PlaceOrderFields")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<PlaceOrderFields, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut contract_combo_legs: Vec<ComboLeg> = vec![];
                let mut order_combo_legs: Vec<OrderComboLeg> = vec![];
                let mut smart_combo_routing_params: Vec<TagValue> = vec![];

                let contract: ContractPreamble = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let trading_class = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                let sec_id_type = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;

                let sec_id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(3, &self))?;

                let ord_hdr: PlaceOrderPreamble = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;

                if contract.sec_type == "BAG" {
                    contract_combo_legs = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(6, &self))?;

                    order_combo_legs = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(8, &self))?;

                    smart_combo_routing_params = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(10, &self))?;
                }

                let shares_alloc_deprecated = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(11, &self))?;

                let discretionary_amt = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(12, &self))?;

                let good_after_time = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(13, &self))?;

                let good_till_date = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(14, &self))?;

                let fa_group = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(15, &self))?;
                let fa_method = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(16, &self))?;
                let fa_percentage = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(17, &self))?;
                let fa_profile = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(18, &self))?;
                let model_code = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(19, &self))?;
                let short_sale_slot = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(20, &self))?;
                let designated_location = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(21, &self))?;
                let exempt_code = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(22, &self))?;
                let oca_type = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(23, &self))?;
                let rule80a = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(24, &self))?;
                let settling_firm = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(25, &self))?;
                let all_or_none = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(26, &self))?;
                let min_qty = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(27, &self))?;
                let percent_offset = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(28, &self))?;
                let e_trade_only = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(29, &self))?;
                let firm_quote_only = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(30, &self))?;
                let nbbo_price_cap = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(31, &self))?;
                let auction_strategy = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(32, &self))?;
                let starting_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(33, &self))?;
                let stock_ref_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(34, &self))?;
                let delta = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(35, &self))?;
                let stock_range_lower = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(36, &self))?;
                let stock_range_upper = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(37, &self))?;
                let override_percentage_constraints = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(38, &self))?;
                let volat = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(39, &self))?;
                let continuous_update = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(40, &self))?;
                let reference_price_type = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(41, &self))?;
                let trail_stop_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(42, &self))?;
                let trailing_percent = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(42, &self))?;
                let scale_init_level_size = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(43, &self))?;
                let scale_subs_level_size = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(44, &self))?;
                let scale_price_increment = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(45, &self))?;

                let mut scale_price_adjust_value = UNSET_DOUBLE;
                let mut scale_price_adjust_interval = UNSET_INTEGER;
                let mut scale_profit_offset = UNSET_DOUBLE;
                let mut scale_auto_reset = false;
                let mut scale_init_position = UNSET_INTEGER;
                let mut scale_init_fill_qty = UNSET_INTEGER;
                let mut scale_random_percent = false;

                if scale_price_increment > 0.0 && scale_price_increment != UNSET_DOUBLE {
                    scale_price_adjust_value = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(46, &self))?;
                    scale_price_adjust_interval = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(47, &self))?;
                    scale_profit_offset = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(48, &self))?;
                    scale_auto_reset = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(49, &self))?;
                    scale_init_position = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(50, &self))?;
                    scale_init_fill_qty = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(51, &self))?;
                    scale_random_percent = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(52, &self))?;
                }
                let scale_table = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(53, &self))?;
                let active_start_time = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(54, &self))?;
                let active_stop_time = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(55, &self))?;
                let hedge_type: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(56, &self))?;
                let mut hedge_param = "".to_string();
                if !hedge_type.is_empty() {
                    hedge_param = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(57, &self))?;
                }
                let opt_out_smart_routing = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(58, &self))?;
                let clearing_account = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(59, &self))?;
                let clearing_intent = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(60, &self))?;
                let not_held = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(61, &self))?;

                let delta_neutral_contract: Option<DeltaNeutralContract> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(62, &self))?;

                let algo_strategy: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(63, &self))?;
                let mut algo_params: Vec<TagValue> = vec![];
                if !algo_strategy.is_empty() {
                    algo_params = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(64, &self))?;
                }
                let algo_id = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(65, &self))?;
                let what_if = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(66, &self))?;
                let misc_options = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(67, &self))?;
                let solicited = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(68, &self))?;
                let randomize_size = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(69, &self))?;
                let randomize_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(70, &self))?;
                let mut reference_contract_id = UNSET_INTEGER;
                let mut is_pegged_change_amount_decrease = false;
                let mut pegged_change_amount = UNSET_DOUBLE;
                let mut reference_change_amount = UNSET_DOUBLE;
                let mut reference_exchange_id = "".to_string();

                if ord_hdr.order_type == "PEG BENCH" {
                    reference_contract_id = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                    is_pegged_change_amount_decrease = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                    pegged_change_amount = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                    reference_change_amount = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                    reference_exchange_id = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                }
                let conditions: Vec<OrderConditionEnum> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let mut conditions_ignore_rth = false;
                let mut conditions_cancel_order = false;
                if conditions.len() > 0 {
                    conditions_ignore_rth = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                    conditions_cancel_order = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                }
                let adjusted_order_type = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let trigger_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let lmt_price_offset = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let adjusted_stop_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let adjusted_stop_limit_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let adjusted_trailing_amount = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let adjustable_trailing_unit = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let ext_operator = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let soft_dollar_tier = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let cash_qty = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let mifid2decision_maker = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let mifid2decision_algo = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let mifid2execution_trader = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let mifid2execution_algo = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let dont_use_auto_price_for_hedge = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let is_oms_container = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let discretionary_up_to_limit_price = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;
                let use_price_mgmt_algo = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(4, &self))?;

                Ok(PlaceOrderFields {
                    contract,
                    trading_class,
                    sec_id_type,
                    sec_id,
                    ord_hdr,
                    contract_combo_legs,
                    order_combo_legs,
                    smart_combo_routing_params,
                    shares_alloc_deprecated,
                    discretionary_amt,
                    good_after_time,
                    good_till_date,
                    fa_group,
                    fa_method,
                    fa_percentage,
                    fa_profile,
                    model_code,
                    short_sale_slot,
                    designated_location,
                    exempt_code,
                    oca_type,
                    rule80a,
                    settling_firm,
                    all_or_none,
                    min_qty,
                    percent_offset,
                    e_trade_only,
                    firm_quote_only,
                    nbbo_price_cap,
                    auction_strategy,
                    starting_price,
                    stock_ref_price,
                    delta,
                    stock_range_lower,
                    stock_range_upper,
                    override_percentage_constraints,
                    volat,
                    continuous_update,
                    reference_price_type,
                    trail_stop_price,
                    trailing_percent,
                    scale_init_level_size,
                    scale_subs_level_size,
                    scale_price_increment,
                    scale_price_adjust_value,
                    scale_price_adjust_interval,
                    scale_profit_offset,
                    scale_auto_reset,
                    scale_init_position,
                    scale_init_fill_qty,
                    scale_random_percent,
                    scale_table,
                    active_start_time,
                    active_stop_time,

                    hedge_type,
                    hedge_param,

                    opt_out_smart_routing,
                    clearing_account,
                    clearing_intent,
                    not_held,
                    delta_neutral_contract,
                    algo_strategy,
                    algo_params,
                    algo_id,
                    what_if,
                    misc_options,
                    solicited,
                    randomize_size,
                    randomize_price,
                    reference_contract_id,
                    is_pegged_change_amount_decrease,
                    pegged_change_amount,
                    reference_change_amount,
                    reference_exchange_id,
                    conditions,
                    conditions_ignore_rth,
                    conditions_cancel_order,
                    adjusted_order_type,
                    trigger_price,
                    lmt_price_offset,
                    adjusted_stop_price,
                    adjusted_stop_limit_price,
                    adjusted_trailing_amount,
                    adjustable_trailing_unit,
                    ext_operator,
                    soft_dollar_tier,
                    cash_qty,
                    mifid2decision_maker,
                    mifid2decision_algo,
                    mifid2execution_trader,
                    mifid2execution_algo,
                    dont_use_auto_price_for_hedge,
                    is_oms_container,
                    discretionary_up_to_limit_price,
                    use_price_mgmt_algo,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "contract",
            "trading_class",
            "sec_id_type",
            "sec_id",
            "ord_hdr",
            "contract_combo_legs",
            "order_combo_legs",
            "smart_combo_routing_params",
            "shares_alloc_deprecated",
            "discretionary_amt",
            "good_after_time",
            "good_till_date",
            "fa_group",
            "fa_method",
            "fa_percentage",
            "fa_profile",
            "model_code",
            "short_sale_slot",
            "designated_location",
            "exempt_code",
            "oca_type",
            "rule80a",
            "settling_firm",
            "all_or_none",
            "min_qty",
            "percent_offset",
            "e_trade_only",
            "firm_quote_only",
            "nbbo_price_cap",
            "auction_strategy",
            "starting_price",
            "stock_ref_price",
            "delta",
            "stock_range_lower",
            "stock_range_upper",
            "override_percentage_constraints",
            "volat",
            "continuous_update",
            "reference_price_type",
            "trail_stop_price",
            "trailing_percent",
            "scale_init_level_size",
            "scale_subs_level_size",
            "scale_price_increment",
            "scale_price_adjust_value",
            "scale_price_adjust_interval",
            "scale_profit_offset",
            "scale_auto_reset",
            "scale_init_position",
            "scale_init_fill_qty",
            "scale_random_percent",
            "scale_table",
            "active_start_time",
            "active_stop_time",
            "hedge_type",
            "hedge_param",
            "opt_out_smart_routing",
            "clearing_account",
            "clearing_intent",
            "not_held",
            "delta_neutral_contract",
            "algo_strategy",
            "algo_params",
            "algo_id",
            "what_if",
            "misc_options",
            "solicited",
            "randomize_size",
            "randomize_price",
            "reference_contract_id",
            "is_pegged_change_amount_decrease",
            "pegged_change_amount",
            "reference_change_amount",
            "reference_exchange_id",
            "conditions",
            "conditions_ignore_rth",
            "conditions_cancel_order",
            "adjusted_order_type",
            "trigger_price",
            "lmt_price_offset",
            "adjusted_stop_price",
            "adjusted_stop_limit_price",
            "adjusted_trailing_amount",
            "adjustable_trailing_unit",
            "ext_operator",
            "soft_dollar_tier",
            "cash_qty",
            "mifid2decision_maker",
            "mifid2decision_algo",
            "mifid2execution_trader",
            "mifid2execution_algo",
            "dont_use_auto_price_for_hedge",
            "is_oms_container",
            "discretionary_up_to_limit_price",
            "use_price_mgmt_algo",
        ];
        deserializer.deserialize_struct("PlaceOrderFields", FIELDS, PlaceOrderFieldsVisitor)
    }
}

#[derive(FromPrimitive)]
#[repr(i32)]
pub enum ServerReqMsgDiscriminants {
    ReqMktData = 1,
    CancelMktData = 2,
    PlaceOrder = 3,
    CancelOrder = 4,
    ReqOpenOrders = 5,
    ReqAcctData = 6,
    ReqExecutions = 7,
    ReqIds = 8,
    ReqContractData = 9,
    ReqMktDepth = 10,
    CancelMktDepth = 11,
    ReqNewsBulletins = 12,
    CancelNewsBulletins = 13,
    SetServerLoglevel = 14,
    ReqAutoOpenOrders = 15,
    ReqAllOpenOrders = 16,
    ReqManagedAccts = 17,
    ReqFa = 18,
    ReplaceFa = 19,
    ReqHistoricalData = 20,
    ExerciseOptions = 21,
    ReqScannerSubscription = 22,
    CancelScannerSubscription = 23,
    ReqScannerParameters = 24,
    CancelHistoricalData = 25,
    ReqCurrentTime = 49,
    ReqRealTimeBars = 50,
    CancelRealTimeBars = 51,
    ReqFundamentalData = 52,
    CancelFundamentalData = 53,
    ReqCalcImpliedVolat = 54,
    ReqCalcOptionPrice = 55,
    CancelCalcImpliedVolat = 56,
    CancelCalcOptionPrice = 57,
    ReqGlobalCancel = 58,
    ReqMarketDataType = 59,
    ReqPositions = 61,
    ReqAccountSummary = 62,
    CancelAccountSummary = 63,
    CancelPositions = 64,
    VerifyRequest = 65,
    VerifyMessage = 66,
    QueryDisplayGroups = 67,
    SubscribeToGroupEvents = 68,
    UpdateDisplayGroup = 69,
    UnsubscribeFromGroupEvents = 70,
    StartApi = 71,
    VerifyAndAuthRequest = 72,
    VerifyAndAuthMessage = 73,
    ReqPositionsMulti = 74,
    CancelPositionsMulti = 75,
    ReqAccountUpdatesMulti = 76,
    CancelAccountUpdatesMulti = 77,
    ReqSecDefOptParams = 78,
    ReqSoftDollarTiers = 79,
    ReqFamilyCodes = 80,
    ReqMatchingSymbols = 81,
    ReqMktDepthExchanges = 82,
    ReqSmartComponents = 83,
    ReqNewsArticle = 84,
    ReqNewsProviders = 85,
    ReqHistoricalNews = 86,
    ReqHeadTimestamp = 87,
    ReqHistogramData = 88,
    CancelHistogramData = 89,
    CancelHeadTimestamp = 90,
    ReqMarketRule = 91,
    ReqPnl = 92,
    CancelPnl = 93,
    ReqPnlSingle = 94,
    CancelPnlSingle = 95,
    ReqHistoricalTicks = 96,
    ReqTickByTickData = 97,
    CancelTickByTickData = 98,
    ReqCompletedOrders = 99,
}

#[derive(Clone, Deserialize, Debug, Display)]
pub enum ServerReqMsg {
    ReqMktData {
        version: i32,
        req_id: i32,
        payload: ReqMktDataFields,
    },
    CancelMktData {
        version: i32,
        req_id: i32,
    },
    PlaceOrder {
        version: i32,
        order_id: i32,
        payload: PlaceOrderFields,
    },
    CancelOrder {
        version: i32,
        order_id: i32,
    },
    ReqOpenOrders {
        version: i32,
    },
    ReqAcctData {
        version: i32,
        subscribe: bool,
        acct_code: String,
    },
    ReqExecutions {
        version: i32,
        req_id: i32,
        exec_filter: ExecutionFilter,
    },
    ReqIds {
        version: i32,
        num_ids: i32,
    },
    ReqContractData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        sec_id_type: String,
        sec_id: String,
    },
    ReqMktDepth {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        num_rows: i32,
        is_smart_depth: bool,
        mkt_depth_options: String,
    },
    CancelMktDepth {
        version: i32,
        req_id: i32,
        is_smart_depth: bool,
    },
    ReqNewsBulletins {
        version: i32,
        all_msgs: bool,
    },
    CancelNewsBulletins {
        version: i32,
    },
    SetServerLoglevel {
        version: i32,
        log_level: i32,
    },
    ReqAutoOpenOrders {
        version: i32,
        auto_bind: bool,
    },
    ReqAllOpenOrders {
        version: i32,
    },
    ReqManagedAccts {
        version: i32,
    },
    ReqFa {
        version: i32,
        fa_data: i32,
    },
    ReplaceFa {
        version: i32,
        fa_data: i32,
        cxml: String,
    },
    ReqHistoricalData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        keep_up_to_date: bool,
        chart_options: String,
    },
    ExerciseOptions {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        exercise_action: i32,
        exercise_quantity: i32,
        account: String,
        over_ride: i32,
    },
    ReqScannerSubscription {
        version: i32,
        req_id: i32,
        subscription: ScannerSubscription,
        scanner_subscription_filter: String,
        scanner_subscription_options: String,
    },
    CancelScannerSubscription {
        version: i32,
        req_id: i32,
    },
    ReqScannerParameters {
        version: i32,
    },
    CancelHistoricalData {
        version: i32,
        req_id: i32,
    },
    ReqCurrentTime {
        version: i32,
    },
    ReqRealTimeBars {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        bar_size: i32,
        what_to_show: String,
        use_rth: bool,
        real_time_bars_options: String,
    },
    CancelRealTimeBars {
        version: i32,
        req_id: i32,
    },
    ReqFundamentalData {
        version: i32,
        req_id: i32,
        contract: ContractPreamble,
        report_type: String,
        tags_value_count: i32,
        fund_data_opt: String,
    },
    CancelFundamentalData {
        version: i32,
        req_id: i32,
    },
    ReqCalcImpliedVolat {
        version: i32,
        req_id: i32,
        contract: ContractPreamble, // abreviated contract field
        trading_class: String,
        option_price: f64,
        under_price: f64,
        tag_values_cnt: usize,
        impl_vol_opt: String,
    },
    ReqCalcOptionPrice {
        version: i32,
        req_id: i32,
        contract: ContractPreamble, // abreviated contract field
        trading_class: String,
        volatility: f64,
        under_price: f64,
        tag_values_cnt: usize,
        opt_prc_opt: String,
    },
    CancelCalcImpliedVolat {
        version: i32,
        req_id: i32,
    },
    CancelCalcOptionPrice {
        version: i32,
        req_id: i32,
    },
    ReqGlobalCancel {
        version: i32,
    },
    ReqMarketDataType {
        version: i32,
        market_data_type: i32,
    },
    ReqPositions {
        version: i32,
    },
    ReqAccountSummary {
        version: i32,
        req_id: i32,
        group_name: String,
        tags: String,
    },
    CancelAccountSummary {
        version: i32,
        req_id: i32,
    },
    CancelPositions {
        version: i32,
    },
    VerifyRequest {
        version: i32,
        api_name: String,
        api_version: String,
    },
    VerifyMessage {
        version: i32,
        api_data: String,
    },
    QueryDisplayGroups {
        version: i32,
        req_id: i32,
    },
    SubscribeToGroupEvents {
        version: i32,
        req_id: i32,
        group_id: i32,
    },
    UpdateDisplayGroup {
        version: i32,
        req_id: i32,
        contract_info: String,
    },
    UnsubscribeFromGroupEvents {
        version: i32,
        req_id: i32,
    },
    StartApi {
        version: i32,
        client_id: String,
    },
    VerifyAndAuthRequest {
        version: i32,
        api_name: String,
        api_version: String,
        opaque_isv_key: String,
    },
    VerifyAndAuthMessage {
        version: i32,
        api_data: String,
        xyz_response: String,
    },
    ReqPositionsMulti {
        version: i32,
        req_id: i32,
        account: String,
        model_code: String,
    },
    CancelPositionsMulti {
        version: i32,
        req_id: i32,
    },
    ReqAccountUpdatesMulti {
        version: i32,
        req_id: i32,
        account: String,
        model_code: String,
        ledger_and_nlv: bool,
    },
    CancelAccountUpdatesMulti {
        version: i32,
        req_id: i32,
    },
    ReqSecDefOptParams {
        req_id: i32,
        underlying_symbol: String,
        fut_fop_exchange: String,
        underlying_sec_type: String,
        underlying_con_id: i32,
    },
    ReqSoftDollarTiers {
        req_id: i32,
    },
    ReqFamilyCodes,
    ReqMatchingSymbols {
        req_id: i32,
        pattern: String,
    },
    ReqMktDepthExchanges,
    ReqSmartComponents {
        req_id: i32,
        bbo_exchange: String,
    },
    ReqNewsArticle {
        req_id: i32,
        provider_code: String,
        article_id: String,
        news_article_options: String,
    },
    ReqNewsProviders,
    ReqHistoricalNews {
        req_id: i32,
        con_id: i32,
        provider_codes: String,
        start_date_time: String,
        end_date_time: String,
        total_results: i32,
        historical_news_options: String,
    },
    ReqHeadTimestamp {
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        use_rth: i32,
        what_to_show: String,
        format_date: i32,
    },
    ReqHistogramData {
        ticker_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        use_rth: bool,
        time_period: String,
    },
    CancelHistogramData {
        ticker_id: i32,
    },
    CancelHeadTimestamp {
        req_id: i32,
    },
    ReqMarketRule {
        market_rule_id: i32,
    },
    ReqPnl {
        req_id: i32,
        account: String,
        model_code: String,
    },
    CancelPnl {
        req_id: i32,
    },
    ReqPnlSingle {
        req_id: i32,
        account: String,
        model_code: String,
        con_id: i32,
    },
    CancelPnlSingle {
        req_id: i32,
    },
    ReqHistoricalTicks {
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        include_expired: bool,
        start_date_time: String,
        end_date_time: String,
        number_of_ticks: i32,
        what_to_show: String,
        use_rth: i32,
        ignore_size: bool,
        misc_options: String,
    },
    ReqTickByTickData {
        req_id: i32,
        contract: ContractPreamble,
        trading_class: String,
        tick_type: String,
        number_of_ticks: i32,
        ignore_size: bool,
    },
    CancelTickByTickData {
        req_id: i32,
    },
    ReqCompletedOrders {
        api_only: bool,
    },
}

//==================================================================================================
pub fn make_message(msg: &str) -> Result<Vec<u8>, IBKRApiLibError> {
    //let mut buffer = ByteBuffer::new();
    let mut buffer: Vec<u8> = Vec::new();

    buffer.extend_from_slice(&i32::to_be_bytes(msg.len() as i32));

    buffer.write(msg.as_ascii_str().unwrap().as_bytes())?;
    let tmp = buffer.clone();
    //debug!("Message after create: {:?}", buffer);

    let (_size, _msg, _buf) = read_msg(tmp.as_slice())?;
    //debug!("Message read: size:{}, msg:{}, bytes: {:?}", size, msg, buf);

    Ok(tmp)
}

//==================================================================================================
pub fn read_msg<'a>(buf: &[u8]) -> Result<(usize, String, Vec<u8>), IBKRApiLibError> {
    // first the size prefix and then the corresponding msg payload ""

    if buf.len() < 4 {
        debug!("read_msg:  buffer too small!! {:?}", buf.len());
        return Ok((0, String::new(), buf.to_vec()));
    }

    let size = i32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
    //debug!("read_msg: Message size: {:?}", size);

    if buf.len() - 4 >= size {
        let text = String::from_utf8(buf[4..4 + size].to_vec()).unwrap();
        //debug!("read_msg: text in read message: {:?}", text);
        Ok((size, text, buf[4 + size..].to_vec()))
    } else {
        Ok((size, String::new(), buf.to_vec()))
    }
}

//==================================================================================================
pub fn read_fields(buf: &str) -> Vec<String> {
    //msg payload is made of fields terminated/separated by NULL chars
    let a = '\u{0}';
    let mut fields: Vec<&str> = buf.split(a).collect::<Vec<&str>>();
    //debug!("fields.len() in read_fields: {}", fields.len());
    //last one is empty
    fields.remove(fields.len() - 1);

    fields
        .iter()
        .map(|x| String::from(*x))
        .collect::<Vec<String>>()
}

//==================================================================================================
pub fn make_field(val: &dyn Any) -> Result<String, IBKRApiLibError> {
    // debug!("CALLING make_field!!");
    // adds the NULL string terminator
    let mut field = "\0".to_string();
    // bool type is encoded as int
    if let Some(boolval) = val.downcast_ref::<bool>() {
        field = format!("{}\0", *boolval as i32);
    } else if let Some(stringval) = val.downcast_ref::<usize>() {
        field = format!("{}\0", *stringval as i32);
    } else if let Some(stringval) = val.downcast_ref::<f64>() {
        if UNSET_DOUBLE == *stringval {
            field = format!("{}\0", "");
        } else {
            field = format!("{}\0", *stringval as f64);
        }
    } else if let Some(stringval) = val.downcast_ref::<i32>() {
        if UNSET_INTEGER == *stringval {
            field = format!("{}\0", "");
        } else {
            field = format!("{}\0", *stringval as i32);
        }
    } else if let Some(stringval) = val.downcast_ref::<String>() {
        field = format!("{}\0", stringval);
    } else if let Some(stringval) = val.downcast_ref::<&str>() {
        field = format!("{}\0", stringval);
    }

    Ok(field)
}

//==================================================================================================
pub fn make_field_handle_empty(val: &dyn Any) -> Result<String, IBKRApiLibError> {
    if let Some(stringval) = val.downcast_ref::<f64>() {
        if UNSET_DOUBLE == *stringval {
            return make_field(&"");
        }
    } else if let Some(stringval) = val.downcast_ref::<i32>() {
        if UNSET_INTEGER == *stringval {
            return make_field(&"");
        }
    }

    make_field(val)
}
