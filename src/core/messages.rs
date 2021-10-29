//! Functions for processing messages
use std::collections::HashSet;
use std::any::Any;
use std::convert::TryInto;
use std::io::Write;
use std::string::String;
use std::vec::Vec;

use rust_decimal::Decimal;
use std::str::FromStr;
use ascii;
use ascii::AsAsciiStr;

use log::*;
use num_derive::FromPrimitive;

use crate::core::contract::{Contract, ContractDescription, ContractDetails, DeltaNeutralContract};
use crate::core::order::{Order, OrderState, SoftDollarTier};
use crate::core::common::{
    BarData, CommissionReport, DepthMktDataDescription, FaDataType, FamilyCode, HistogramData, HistoricalTick,
    HistoricalTickBidAsk, HistoricalTickLast, NewsProvider, PriceIncrement, RealTimeBar,
    SmartComponent, TagValue, TickAttrib, TickAttribBidAsk, TickAttribLast, TickByTickType, TickType, MAX_MSG_LEN,
    NO_VALID_ID, UNSET_DOUBLE, UNSET_INTEGER,
};
use crate::core::errors::IBKRApiLibError;
use crate::core::execution::Execution;

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

#[derive(Clone, Debug)]
pub enum IncomingMsgCmd {
    ErrorMsg { req_id: i32, error_code: i32, error_str: String }, 
    TickPriceMsg { req_id: i32, tick_type: TickType, price: f64, size: i32, tick_attr: TickAttrib },
    TickSizeMsg { req_id: i32, size_tick_type: i32, size: i32 },
    TickSnapShotEndMsg{ req_id: i32},
    TickGenericMsg { req_id: i32, tick_type: TickType, value: f64},
    TickStringMsg { req_id: i32, tick_type: TickType, value: String },
    TickEFPMsg {
        req_id: i32,
        tick_type: TickType,
        basis_points: f64,
        formatted_basis_points: String,
        implied_future: f64,
        hold_days: i32,
        future_last_trade_date: String,
        dividend_impact: f64,
        dividends_to_last_trade_date: f64,
    },
    OrderStatusMsg {
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
    OpenOrderMsg {
        order_id: i32,
        contract: Contract,
        order: Order,
        order_state: OrderState,
    },
    OpenOrderEndMsg ,
    ConnectionClosedMsg ,
    UpdateAccountValueMsg  { key: String, val: String, currency: String, account_name: String},
    UpdatePortfolioMsg  {
        contract: Contract,
        position: f64,
        market_price: f64,
        market_value: f64,
        average_cost: f64,
        unrealized_pnl: f64,
        realized_pnl: f64,
        account_name: String,
    },
    UpdateAccountTimeMsg  { time_stamp: String},
    AccountDownloadEndMsg  { account_name: String},
    NextValidIdMsg  { order_id: i32},
    ContractDetailsMsg  { req_id: i32, contract_details: ContractDetails},
    BondContractDetailsMsg  { req_id: i32, contract_details: ContractDetails},
    ContractDetailsEndMsg  { req_id: i32},
    ExecDetailsMsg  { req_id: i32, contract: Contract, execution: Execution},
    ExecDetailsEndMsg  { req_id: i32},
    UpdateMarketDepthMsg  {
        req_id: i32,
        position: i32,
        operation: i32,
        side: i32,
        price: f64,
        size: i32,
    },
    UpdateMarketDepthL2Msg  {
        req_id: i32,
        position: i32,
        market_maker: String,
        operation: i32,
        side: i32,
        price: f64,
        size: i32,
        is_smart_depth: bool,
    },
    UpdateNewsBulletinMsg  {
        msg_id: i32,
        msg_type: i32,
        news_message: String,
        origin_exch: String,
    },
    ManagedAccountsMsg  { accounts_list: String},
    MarketDataTypeMsg { req_id: i32, market_data_type: i32 },
    ReceiveFaMsg  { fa_data: FaDataType, cxml: String},
    HistoricalDataMsg  { req_id: i32, bar: BarData},
    HistoricalDataEndMsg  { req_id: i32, start: String, end: String},
    ScannerParametersMsg  { xml: String},
    ScannerDataEndMsg  { req_id: i32},
    RealtimeBarMsg  { req_id: i32, bar: RealTimeBar},
    CurrentTimeMsg  { time: i64},
    FundamentalDataMsg  { req_id: i32, data: String},
    DeltaNeutralValidationMsg  { req_id: i32, delta_neutral_contract: DeltaNeutralContract},
    CommissionReportMsg  { commission_report: CommissionReport},
    PositionMsg  { account: String, contract: Contract, position: f64, avg_cost: f64},
    PositionEndMsg,
    AccountSummaryMsg  { req_id: i32, account: String, tag: String, value: String, currency: String},
    AccountSummaryEndMsg  { req_id: i32},
    VerifyMessageApiMsg  { api_data: String},
    VerifyCompletedMsg  { is_successful: bool, error_text: String},
    VerifyAndAuthMessageApiMsg  { api_data: String, xyz_challange: String},
    VerifyAndAuthCompletedMsg  { is_successful: bool, error_text: String},
    DisplayGroupListMsg  { req_id: i32, groups: String},
    DisplayGroupUpdatedMsg  { req_id: i32, contract_info: String},
    PositionMultiMsg  { req_id: i32, account: String, model_code: String, contract: Contract, pos: f64, avg_cost: f64 },
    PositionMultiEndMsg  { req_id: i32},
    AccountUpdateMultiMsg  { req_id: i32, account: String, model_code: String, key: String, value: String, currency: String },
    AccountUpdateMultiEndMsg  { req_id: i32},
    TickOptionComputationMsg  {
        req_id: i32,
        tick_type: TickType,
        implied_vol: f64,
        delta: f64,
        opt_price: f64,
        pv_dividend: f64,
        gamma: f64,
        vega: f64,
        theta: f64,
        und_price: f64
    },
    SecurityDefinitionOptionParameterMsg  {
        req_id: i32,
        exchange: String,
        underlying_con_id: i32,
        trading_class: String,
        multiplier: String,
        expirations: HashSet<String>,
        strikes: HashSet<Decimal>
    },
    SecurityDefinitionOptionParameterEndMsg  { req_id: i32},
    SoftDollarTiersMsg  { req_id: i32, tiers: Vec<SoftDollarTier>},
    FamilyCodesMsg  { family_codes: Vec<FamilyCode>},
    SymbolSamplesMsg  { req_id: i32, contract_descriptions: Vec<ContractDescription>},
    MarketDepthExchangesMsg  { depth_mkt_data_descriptions: Vec<DepthMktDataDescription>},
    TickNewsMsg  {
        ticker_id: i32,
        time_stamp: i32,
        provider_code: String,
        article_id: String,
        headline: String,
        extra_data: String },
    SmartComponentsMsg  { req_id: i32, smart_components: Vec<SmartComponent>},
    TickReqParamsMsg  {
        ticker_id: i32,
        min_tick: f64,
        bbo_exchange: String,
        snapshot_permissions: i32 },
    NewsProvidersMsg  { news_providers: Vec<NewsProvider>},
    NewsArticleMsg  { req_id: i32, article_type: i32, article_text: String},
    HistoricalNewsMsg  {
        req_id: i32,
        time: String,
        provider_code: String,
        article_id: String,
        headline: String },
    HistoricalNewsEndMsg  { req_id: i32, has_more: bool},
    HeadTimestampMsg  { req_id: i32, head_timestamp: String},
    HistogramDataMsg  { req_id: i32, items: Vec<HistogramData>},
    HistoricalDataUpdateMsg  { req_id: i32, bar: BarData},
    RerouteMarketDataReqMsg  { req_id: i32, con_id: i32, exchange: String},
    RerouteMarketDepthReqMsg  { req_id: i32, con_id: i32, exchange: String},
    MarketRuleMsg  { market_rule_id: i32, price_increments: Vec<PriceIncrement>},
    PnlMsg  { req_id: i32, daily_pn_l: f64, unrealized_pn_l: f64, realized_pn_l: f64},
    PnlSingleMsg  {
        req_id: i32,
        pos: i32,
        daily_pn_l: f64,
        unrealized_pn_l: f64,
        realized_pn_l: f64,
        value: f64 },
    HistoricalTicksMsg  { req_id: i32, ticks: Vec<HistoricalTick>, done: bool},
    HistoricalTicksBidAskMsg  {
        req_id: i32,
        ticks: Vec<HistoricalTickBidAsk>,
        done: bool },
    HistoricalTicksLastMsg  { req_id: i32, ticks: Vec<HistoricalTickLast>, done: bool},
    TickByTickAllLastMsg  {
        req_id: i32,
        tick_type: TickByTickType,
        time: i64,
        price: f64,
        size: i32,
        tick_attrib_last: TickAttribLast,
        exchange: String,
        special_conditions: String },
    TickByTickBidAskMsg  {
        req_id: i32,
        time: i64,
        bid_price: f64,
        ask_price: f64,
        bid_size: i32,
        ask_size: i32,
        tick_attrib_bid_ask: TickAttribBidAsk },
    TickByTickMidPointMsg  { req_id: i32, time: i64, mid_point: f64},
    OrderBoundMsg  { req_id: i32, api_client_id: i32, api_order_id: i32},
    CompletedOrderMsg  { contract: Contract, order: Order, order_state: OrderState},
    CompletedOrdersEndMsg,
}

//==================================================================================================
/// incoming msg id's
#[derive(FromPrimitive)]
#[repr(i32)]
pub enum IncomingMessageIds {
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

//==================================================================================================
/// Outgoing msg id's
#[derive(FromPrimitive)]
#[repr(i32)]
pub enum OutgoingMessageIds {
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
