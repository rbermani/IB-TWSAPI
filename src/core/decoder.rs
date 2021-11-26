//! Receives messages from Reader, decodes messages, and feeds them to Cmd  Queue
use std::collections::HashSet;

use std::ops::Deref;
use std::slice::Iter;
use std::str::FromStr;
use std::string::ToString;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use float_cmp::*;
use log::*;
use num_traits::float::FloatCore;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;

use crate::core::client::ConnStatus;
use crate::core::common::{
    BarData, CommissionReport, DepthMktDataDescription, FamilyCode, HistogramData, HistoricalTick,
    HistoricalTickBidAsk, HistoricalTickLast, NewsProvider, PriceIncrement, RealTimeBar,
    SmartComponent, TagValue, TickAttrib, TickAttribBidAsk, TickAttribLast, TickMsgType, TickType,
    MAX_MSG_LEN, NO_VALID_ID, UNSET_DOUBLE, UNSET_INTEGER,
};
use crate::core::contract::{Contract, ContractDescription, ContractDetails, DeltaNeutralContract};
use crate::core::errors::{IBKRApiLibError, TwsError};
use crate::core::execution::Execution;
use crate::core::messages::{read_fields, ServerRspMsg, ServerRspMsgDiscriminants};
use crate::core::order::{Order, OrderState, SoftDollarTier};
use crate::core::order_decoder::OrderDecoder;
use crate::core::scanner::ScanData;
use crate::core::server_versions::{
    MIN_SERVER_VER_AGG_GROUP, MIN_SERVER_VER_FRACTIONAL_POSITIONS, MIN_SERVER_VER_LAST_LIQUIDITY,
    MIN_SERVER_VER_MARKET_CAP_PRICE, MIN_SERVER_VER_MARKET_RULES,
    MIN_SERVER_VER_MD_SIZE_MULTIPLIER, MIN_SERVER_VER_MODELS_SUPPORT,
    MIN_SERVER_VER_ORDER_CONTAINER, MIN_SERVER_VER_PAST_LIMIT, MIN_SERVER_VER_PRE_OPEN_BID_ASK,
    MIN_SERVER_VER_REALIZED_PNL, MIN_SERVER_VER_REAL_EXPIRATION_DATE,
    MIN_SERVER_VER_SERVICE_DATA_TYPE, MIN_SERVER_VER_SMART_DEPTH,
    MIN_SERVER_VER_SYNT_REALTIME_BARS, MIN_SERVER_VER_UNDERLYING_INFO,
    MIN_SERVER_VER_UNREALIZED_PNL,
};

//==================================================================================================
pub fn decode_i32(iter: &mut Iter<String>) -> Result<i32, IBKRApiLibError> {
    let next = iter.next();

    let val: i32 = next.unwrap().parse().unwrap_or(0);
    Ok(val)
}

//==================================================================================================
pub fn decode_tick_type(iter: &mut Iter<String>) -> Result<TickType, IBKRApiLibError> {
    let next = iter.next();

    let val: TickType = next.unwrap().parse().unwrap_or(TickType::NotSet);
    Ok(val)
}

//==================================================================================================
pub fn decode_i32_show_unset(iter: &mut Iter<String>) -> Result<i32, IBKRApiLibError> {
    let next = iter.next();
    //info!("{:?}", next);
    let retval: i32 = next.unwrap().parse().unwrap_or(0);
    Ok(if retval == 0 { UNSET_INTEGER } else { retval })
}

//==================================================================================================
pub fn decode_i64(iter: &mut Iter<String>) -> Result<i64, IBKRApiLibError> {
    let next = iter.next();
    //info!("{:?}", next);
    let val: i64 = next.unwrap().parse().unwrap_or(0);
    Ok(val)
}

//==================================================================================================
pub fn decode_f64(iter: &mut Iter<String>) -> Result<f64, IBKRApiLibError> {
    let next = iter.next();
    //info!("{:?}", next);
    let val = next.unwrap().parse().unwrap_or(0.0);
    Ok(val)
}

//==================================================================================================
pub fn decode_f64_show_unset(iter: &mut Iter<String>) -> Result<f64, IBKRApiLibError> {
    let next = iter.next();
    //info!("{:?}", next);
    let retval: f64 = next.unwrap().parse().unwrap_or(0.0);
    Ok(if retval == 0.0 { UNSET_DOUBLE } else { retval })
}

//==================================================================================================
pub fn decode_string(iter: &mut Iter<String>) -> Result<String, IBKRApiLibError> {
    let next = iter.next();
    //info!("{:?}", next);
    let val = next.unwrap().parse().unwrap_or("".to_string());
    Ok(val)
}

//==================================================================================================
pub fn decode_bool(iter: &mut Iter<String>) -> Result<bool, IBKRApiLibError> {
    let next = iter.next();
    //info!("{:?}", next);
    let retval: i32 = next.unwrap_or(&"0".to_string()).parse().unwrap_or(0);
    Ok(retval != 0)
}

//==================================================================================================
pub struct Decoder {
    msg_queue: Receiver<String>,
    send_queue: Sender<ServerRspMsg>,
    pub server_version: i32,
    conn_state: Arc<Mutex<ConnStatus>>,
}

impl Decoder {
    pub fn new(
        msg_queue: Receiver<String>,
        send_queue: Sender<ServerRspMsg>,
        server_version: i32,
        conn_state: Arc<Mutex<ConnStatus>>,
    ) -> Self {
        Decoder {
            send_queue: send_queue,
            msg_queue: msg_queue,
            server_version,
            conn_state,
        }
    }

    //----------------------------------------------------------------------------------------------
    pub fn interpret(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        if fields.is_empty() {
            return Ok(());
        }

        let msg_id = i32::from_str(fields.get(0).unwrap().as_str())?;

        match FromPrimitive::from_i32(msg_id) {
            Some(ServerRspMsgDiscriminants::TickPrice) => self.process_tick_price(fields)?,
            Some(ServerRspMsgDiscriminants::AccountSummary) => {
                self.process_account_summary(fields)?
            }
            Some(ServerRspMsgDiscriminants::AccountSummaryEnd) => {
                self.process_account_summary_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::AccountUpdateMulti) => {
                self.process_account_update_multi(fields)?
            }
            Some(ServerRspMsgDiscriminants::AccountUpdateMultiEnd) => {
                self.process_account_update_multi_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::AcctDownloadEnd) => {
                self.process_account_download_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::AcctUpdateTime) => {
                self.process_account_update_time(fields)?
            }
            Some(ServerRspMsgDiscriminants::AcctValue) => self.process_account_value(fields)?,
            Some(ServerRspMsgDiscriminants::BondContractData) => {
                self.process_bond_contract_data(fields)?
            }
            Some(ServerRspMsgDiscriminants::CommissionReport) => {
                self.process_commission_report(fields)?
            }
            Some(ServerRspMsgDiscriminants::CompletedOrder) => {
                self.process_completed_order(fields)?
            }
            Some(ServerRspMsgDiscriminants::CompletedOrdersEnd) => {
                self.process_end_msg_noarg(ServerRspMsg::CompletedOrdersEnd)?
            }
            Some(ServerRspMsgDiscriminants::ContractData) => {
                self.process_contract_details(fields)?
            }
            Some(ServerRspMsgDiscriminants::ContractDataEnd) => {
                self.process_contract_details_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::CurrentTime) => self.process_current_time(fields)?,
            Some(ServerRspMsgDiscriminants::DeltaNeutralValidation) => {
                self.process_delta_neutral_validation(fields)?
            }
            Some(ServerRspMsgDiscriminants::DisplayGroupList) => {
                self.process_display_group_list(fields)?
            }
            Some(ServerRspMsgDiscriminants::DisplayGroupUpdated) => {
                self.process_display_group_updated(fields)?
            }
            Some(ServerRspMsgDiscriminants::ErrMsg) => self.process_error_message(fields)?,
            Some(ServerRspMsgDiscriminants::ExecutionData) => {
                self.process_execution_data(fields)?
            }
            Some(ServerRspMsgDiscriminants::ExecutionDataEnd) => {
                self.process_execution_data_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::FamilyCodes) => self.process_family_codes(fields)?,
            Some(ServerRspMsgDiscriminants::FundamentalData) => {
                self.process_fundamental_data(fields)?
            }
            Some(ServerRspMsgDiscriminants::HeadTimestamp) => {
                self.process_head_timestamp(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistogramData) => {
                self.process_histogram_data(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistoricalData) => {
                self.process_historical_data(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistoricalDataUpdate) => {
                self.process_historical_data_update(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistoricalNews) => {
                self.process_historical_news(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistoricalNewsEnd) => {
                self.process_historical_news_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistoricalTicks) => {
                self.process_historical_ticks(fields)?
            }
            Some(ServerRspMsgDiscriminants::HistoricalTicksBidAsk) => {
                self.process_historical_ticks_bid_ask(fields)?
            }

            Some(ServerRspMsgDiscriminants::HistoricalTicksLast) => {
                self.process_historical_ticks_last(fields)?
            }
            Some(ServerRspMsgDiscriminants::ManagedAccts) => {
                self.process_managed_accounts(fields)?
            }
            Some(ServerRspMsgDiscriminants::MarketDataType) => {
                self.process_market_data_type(fields)?
            }
            Some(ServerRspMsgDiscriminants::MarketDepth) => self.process_market_depth(fields)?,
            Some(ServerRspMsgDiscriminants::MarketDepthL2) => {
                self.process_market_depth_l2(fields)?
            }
            Some(ServerRspMsgDiscriminants::MarketRule) => self.process_market_rule(fields)?,
            Some(ServerRspMsgDiscriminants::MktDepthExchanges) => {
                self.process_market_depth_exchanges(fields)?
            }
            Some(ServerRspMsgDiscriminants::NewsArticle) => self.process_news_article(fields)?,
            Some(ServerRspMsgDiscriminants::NewsBulletins) => {
                self.process_news_bulletins(fields)?
            }
            Some(ServerRspMsgDiscriminants::NewsProviders) => {
                self.process_news_providers(fields)?
            }
            Some(ServerRspMsgDiscriminants::NextValidId) => self.process_next_valid_id(fields)?,
            Some(ServerRspMsgDiscriminants::OpenOrder) => self.process_open_order(fields)?,
            Some(ServerRspMsgDiscriminants::OpenOrderEnd) => {
                self.process_end_msg_noarg(ServerRspMsg::OpenOrderEnd)?
            }
            Some(ServerRspMsgDiscriminants::OrderStatus) => self.process_order_status(fields)?,
            Some(ServerRspMsgDiscriminants::OrderBound) => self.process_order_bound(fields)?,
            Some(ServerRspMsgDiscriminants::Pnl) => self.process_pnl(fields)?,
            Some(ServerRspMsgDiscriminants::PnlSingle) => self.process_pnl_single(fields)?,
            Some(ServerRspMsgDiscriminants::PortfolioValue) => {
                self.process_portfolio_value(fields)?
            }
            Some(ServerRspMsgDiscriminants::PositionData) => self.process_position_data(fields)?,
            Some(ServerRspMsgDiscriminants::PositionEnd) => {
                self.process_end_msg_noarg(ServerRspMsg::PositionEnd)?
            }
            Some(ServerRspMsgDiscriminants::RealTimeBars) => self.process_real_time_bars(fields)?,
            Some(ServerRspMsgDiscriminants::ReceiveFa) => self.process_receive_fa(fields)?,
            Some(ServerRspMsgDiscriminants::RerouteMktDataReq) => {
                self.process_reroute_mkt_data_req(fields)?
            }

            Some(ServerRspMsgDiscriminants::PositionMulti) => {
                self.process_position_multi(fields)?
            }
            Some(ServerRspMsgDiscriminants::PositionMultiEnd) => {
                self.process_position_multi_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::ScannerData) => self.process_scanner_data(fields)?,
            Some(ServerRspMsgDiscriminants::ScannerParameters) => {
                self.process_scanner_parameters(fields)?
            }
            Some(ServerRspMsgDiscriminants::SecurityDefinitionOptionParameter) => {
                self.process_security_definition_option_parameter(fields)?
            }
            Some(ServerRspMsgDiscriminants::SecurityDefinitionOptionParameterEnd) => {
                self.process_security_definition_option_parameter_end(fields)?
            }

            Some(ServerRspMsgDiscriminants::SmartComponents) => {
                self.process_smart_components(fields)?
            }
            Some(ServerRspMsgDiscriminants::SoftDollarTiers) => {
                self.process_soft_dollar_tiers(fields)?
            }
            Some(ServerRspMsgDiscriminants::SymbolSamples) => {
                self.process_symbol_samples(fields)?
            }
            Some(ServerRspMsgDiscriminants::TickByTick) => self.process_tick_by_tick(fields)?,
            Some(ServerRspMsgDiscriminants::TickEfp) => self.process_tick_by_tick(fields)?,
            Some(ServerRspMsgDiscriminants::TickGeneric) => self.process_tick_generic(fields)?,
            Some(ServerRspMsgDiscriminants::TickNews) => self.process_tick_news(fields)?,
            Some(ServerRspMsgDiscriminants::TickOptionComputation) => {
                self.process_tick_option_computation(fields)?
            }
            Some(ServerRspMsgDiscriminants::TickReqParams) => {
                self.process_tick_req_params(fields)?
            }
            Some(ServerRspMsgDiscriminants::TickSize) => self.process_tick_size(fields)?,
            Some(ServerRspMsgDiscriminants::TickSnapshotEnd) => {
                self.process_tick_snapshot_end(fields)?
            }
            Some(ServerRspMsgDiscriminants::TickString) => self.process_tick_string(fields)?,
            Some(ServerRspMsgDiscriminants::VerifyAndAuthCompleted) => {
                self.process_verify_and_auth_completed(fields)?
            }

            Some(ServerRspMsgDiscriminants::VerifyCompleted) => {
                self.process_verify_completed(fields)?
            }

            Some(ServerRspMsgDiscriminants::VerifyMessageApi) => {
                self.process_verify_completed(fields)?
            }

            Some(ServerRspMsgDiscriminants::VerifyAndAuthMessageApi) => {
                self.process_verify_and_auth_message_api(fields)?
            }
            Some(ServerRspMsgDiscriminants::RerouteMktDepthReq) => {
                self.process_reroute_mkt_depth_req(fields)?
            }

            _ => panic!("Received unkown message id!!  Exiting..."),
        }
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_price(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let req_id: i32 = decode_i32(&mut fields_itr)?;
        let tick_type_i32: i32 = decode_i32(&mut fields_itr)?;
        let price: f64 = decode_f64(&mut fields_itr)?;
        let size: i32 = decode_i32(&mut fields_itr)?;
        let attr: i32 = decode_i32(&mut fields_itr)?;

        let mut tick_attrib = TickAttrib::new(false, false, false);

        tick_attrib.can_auto_execute = attr == 1;

        if self.server_version >= MIN_SERVER_VER_PAST_LIMIT {
            tick_attrib.can_auto_execute = attr & 1 != 0;
            tick_attrib.past_limit = attr & 2 != 0;
        }
        if self.server_version >= MIN_SERVER_VER_PRE_OPEN_BID_ASK {
            tick_attrib.pre_open = attr & 4 != 0;
        }

        let tick_price = ServerRspMsg::TickPrice {
            req_id: req_id,
            tick_type: FromPrimitive::from_i32(tick_type_i32).unwrap(),
            price: price,
            tick_attr: tick_attrib.clone(),
        };

        self.send_queue.send(tick_price.clone()).unwrap();

        if let ServerRspMsg::TickPrice { .. } = tick_price {
            // process ver 2 fields
            let size_tick_type = match FromPrimitive::from_i32(tick_type_i32) {
                Some(TickType::Bid) => TickType::BidSize,
                Some(TickType::Ask) => TickType::AskSize,
                Some(TickType::Last) => TickType::LastSize,
                Some(TickType::DelayedBid) => TickType::DelayedBidSize,
                Some(TickType::DelayedAsk) => TickType::DelayedAskSize,
                Some(TickType::DelayedLast) => TickType::DelayedLastSize,
                _ => TickType::NotSet,
            };

            if size_tick_type as i32 != TickType::NotSet as i32 {
                let tick_size = ServerRspMsg::TickSize {
                    req_id: req_id,
                    tick_type: size_tick_type,
                    size: size,
                };
                self.send_queue.send(tick_size).unwrap();
            }
        }

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_string(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let tick_string = ServerRspMsg::TickString {
            req_id: decode_i32(&mut fields_itr)?,
            tick_type: decode_tick_type(&mut fields_itr)?,
            value: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(tick_string).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_summary(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let account_summary = ServerRspMsg::AccountSummary {
            req_id: decode_i32(&mut fields_itr)?,
            account: decode_string(&mut fields_itr)?,
            tag: decode_string(&mut fields_itr)?,
            value: decode_string(&mut fields_itr)?,
            currency: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(account_summary).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_summary_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let account_summary_end = ServerRspMsg::AccountSummaryEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(account_summary_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_update_multi(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let account_update_multi = ServerRspMsg::AccountUpdateMulti {
            req_id: decode_i32(&mut fields_itr)?,
            account: decode_string(&mut fields_itr)?,
            model_code: decode_string(&mut fields_itr)?,
            key: decode_string(&mut fields_itr)?,
            value: decode_string(&mut fields_itr)?,
            currency: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(account_update_multi).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_update_multi_end(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let account_update_multi_end = ServerRspMsg::AccountUpdateMultiEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(account_update_multi_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_download_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let account_download_end = ServerRspMsg::AcctDownloadEnd {
            account_name: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(account_download_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_update_time(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let update_account_time = ServerRspMsg::AcctUpdateTime {
            time_stamp: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(update_account_time).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_account_value(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let update_account_value = ServerRspMsg::AcctValue {
            key: decode_string(&mut fields_itr)?,
            val: decode_string(&mut fields_itr)?,
            currency: decode_string(&mut fields_itr)?,
            account_name: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(update_account_value).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_bond_contract_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let version: i32 = decode_i32(&mut fields_itr)?;

        let mut req_id = -1;
        if version >= 3 {
            req_id = decode_i32(&mut fields_itr)?;
        }

        let mut contract = ContractDetails::default();

        contract.contract.symbol = decode_string(&mut fields_itr)?;
        contract.contract.sec_type = decode_string(&mut fields_itr)?;
        contract.cusip = decode_string(&mut fields_itr)?;
        contract.coupon = decode_f64(&mut fields_itr)?;
        self.read_last_trade_date(&mut contract, true, fields_itr.next().unwrap())?;
        contract.issue_date = decode_string(&mut fields_itr)?;
        contract.ratings = decode_string(&mut fields_itr)?;
        contract.bond_type = decode_string(&mut fields_itr)?;
        contract.coupon_type = decode_string(&mut fields_itr)?;
        contract.convertible = i32::from_str(fields_itr.next().unwrap().as_ref())? != 0;
        contract.callable = i32::from_str(fields_itr.next().unwrap().as_ref())? != 0;
        contract.putable = i32::from_str(fields_itr.next().unwrap().as_ref())? != 0;
        contract.desc_append = decode_string(&mut fields_itr)?;
        contract.contract.exchange = decode_string(&mut fields_itr)?;
        contract.contract.currency = decode_string(&mut fields_itr)?;
        contract.market_name = decode_string(&mut fields_itr)?;
        contract.contract.trading_class = decode_string(&mut fields_itr)?;
        contract.contract.con_id = decode_i32(&mut fields_itr)?;
        contract.min_tick = decode_f64(&mut fields_itr)?;
        if self.server_version >= MIN_SERVER_VER_MD_SIZE_MULTIPLIER {
            contract.md_size_multiplier = decode_i32(&mut fields_itr)?;
        }
        contract.order_types = decode_string(&mut fields_itr)?;
        contract.valid_exchanges = decode_string(&mut fields_itr)?;
        if version >= 2 {
            contract.next_option_date = decode_string(&mut fields_itr)?;
            contract.next_option_type = decode_string(&mut fields_itr)?;
            contract.next_option_partial = decode_bool(&mut fields_itr)?;
            contract.notes = decode_string(&mut fields_itr)?;
        }
        if version >= 4 {
            contract.long_name = decode_string(&mut fields_itr)?;
        }
        if version >= 6 {
            contract.ev_rule = decode_string(&mut fields_itr)?;
            contract.ev_multiplier = decode_f64(&mut fields_itr)?;
        }
        if version >= 5 {
            let sec_id_list_count = decode_i32(&mut fields_itr)?;
            if sec_id_list_count > 0 {
                contract.sec_id_list = vec![];
                for _ in 0..sec_id_list_count {
                    contract.sec_id_list.push(TagValue::new(
                        fields_itr.next().unwrap().parse().unwrap(),
                        fields_itr.next().unwrap().parse().unwrap(),
                    ));
                }
            }
        }
        if self.server_version >= MIN_SERVER_VER_AGG_GROUP {
            contract.agg_group = decode_i32(&mut fields_itr)?;
        }
        if self.server_version >= MIN_SERVER_VER_MARKET_RULES {
            contract.market_rule_ids = decode_string(&mut fields_itr)?;
        }

        let bond_contract_details = ServerRspMsg::BondContractData {
            req_id: req_id,
            contract_details: contract.clone(),
        };

        self.send_queue.send(bond_contract_details).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_commission_report(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let mut commission_report = CommissionReport::default();
        commission_report.exec_id = fields_itr.next().unwrap().to_string();
        commission_report.commission = decode_f64(&mut fields_itr)?;
        commission_report.currency = fields_itr.next().unwrap().to_string();
        commission_report.realized_pnl = decode_f64(&mut fields_itr)?;
        commission_report.yield_ = decode_f64(&mut fields_itr)?;
        commission_report.yield_redemption_date = decode_string(&mut fields_itr)?;

        let commission_report = ServerRspMsg::CommissionReport {
            commission_report: commission_report.clone(),
        };

        self.send_queue.send(commission_report).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_completed_order(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let mut contract = Contract::default();
        let mut order = Order::default();
        let mut order_state = OrderState::default();

        let mut order_decoder = OrderDecoder::new(
            &mut contract,
            &mut order,
            &mut order_state,
            UNSET_INTEGER,
            self.server_version,
        );

        order_decoder.decode_completed(&mut fields_itr)?;

        let completed_order = ServerRspMsg::CompletedOrder {
            contract: contract.clone(),
            order: order.clone(),
            order_state: order_state.clone(),
        };

        self.send_queue.send(completed_order).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_contract_details(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let version: i32 = decode_i32(&mut fields_itr)?;

        let mut req_id = -1;
        if version >= 3 {
            req_id = decode_i32(&mut fields_itr)?;
        }

        let mut contract = ContractDetails::default();

        contract.contract.symbol = decode_string(&mut fields_itr)?;
        contract.contract.sec_type = decode_string(&mut fields_itr)?;
        self.read_last_trade_date(&mut contract, false, fields_itr.next().unwrap())?;
        contract.contract.strike = decode_f64(&mut fields_itr)?;
        contract.contract.right = decode_string(&mut fields_itr)?;
        contract.contract.exchange = decode_string(&mut fields_itr)?;
        contract.contract.currency = decode_string(&mut fields_itr)?;
        contract.contract.local_symbol = decode_string(&mut fields_itr)?;
        contract.market_name = decode_string(&mut fields_itr)?;
        contract.contract.trading_class = decode_string(&mut fields_itr)?;
        contract.contract.con_id = decode_i32(&mut fields_itr)?;
        contract.min_tick = decode_f64(&mut fields_itr)?;
        if self.server_version >= MIN_SERVER_VER_MD_SIZE_MULTIPLIER {
            contract.md_size_multiplier = decode_i32(&mut fields_itr)?;
        }
        contract.contract.multiplier = decode_string(&mut fields_itr)?;
        contract.order_types = decode_string(&mut fields_itr)?;
        contract.valid_exchanges = decode_string(&mut fields_itr)?;
        contract.price_magnifier = decode_i32(&mut fields_itr)?;
        if version >= 4 {
            contract.under_con_id = decode_i32(&mut fields_itr)?;
        }
        if version >= 5 {
            contract.long_name = decode_string(&mut fields_itr)?;
            contract.contract.primary_exchange = decode_string(&mut fields_itr)?;
        }

        if version >= 6 {
            contract.contract_month = decode_string(&mut fields_itr)?;
            contract.industry = decode_string(&mut fields_itr)?;
            contract.category = decode_string(&mut fields_itr)?;
            contract.subcategory = decode_string(&mut fields_itr)?;
            contract.time_zone_id = decode_string(&mut fields_itr)?;
            contract.trading_hours = decode_string(&mut fields_itr)?;
            contract.liquid_hours = decode_string(&mut fields_itr)?;
        }
        if version >= 8 {
            contract.ev_rule = decode_string(&mut fields_itr)?;
            contract.ev_multiplier = decode_f64(&mut fields_itr)?;
        }

        if version >= 7 {
            let sec_id_list_count = decode_i32(&mut fields_itr)?;
            if sec_id_list_count > 0 {
                contract.sec_id_list = vec![];
                for _ in 0..sec_id_list_count {
                    contract.sec_id_list.push(TagValue::new(
                        decode_string(&mut fields_itr)?,
                        decode_string(&mut fields_itr)?,
                    ));
                }
            }
        }
        if self.server_version >= MIN_SERVER_VER_AGG_GROUP {
            contract.agg_group = decode_i32(&mut fields_itr)?;
        }

        if self.server_version >= MIN_SERVER_VER_UNDERLYING_INFO {
            contract.under_symbol = decode_string(&mut fields_itr)?;
            contract.under_sec_type = decode_string(&mut fields_itr)?;
        }
        if self.server_version >= MIN_SERVER_VER_MARKET_RULES {
            contract.market_rule_ids = decode_string(&mut fields_itr)?;
        }

        if self.server_version >= MIN_SERVER_VER_REAL_EXPIRATION_DATE {
            contract.real_expiration_date = decode_string(&mut fields_itr)?;
        }

        let contract_details = ServerRspMsg::ContractData {
            req_id: req_id,
            contract_details: contract.clone(),
        };

        self.send_queue.send(contract_details).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_contract_details_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let contract_details_end = ServerRspMsg::ContractDataEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(contract_details_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_current_time(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let current_time = ServerRspMsg::CurrentTime {
            time: decode_i64(&mut fields_itr)?,
        };

        self.send_queue.send(current_time).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_delta_neutral_validation(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let mut delta_neutral_contract = DeltaNeutralContract::default();

        delta_neutral_contract.con_id = decode_i32(&mut fields_itr)?;
        delta_neutral_contract.delta = decode_f64(&mut fields_itr)?;
        delta_neutral_contract.price = decode_f64(&mut fields_itr)?;

        let delta_neutral_validation = ServerRspMsg::DeltaNeutralValidation {
            req_id: req_id,
            delta_neutral_contract: delta_neutral_contract.clone(),
        };

        self.send_queue.send(delta_neutral_validation).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_display_group_list(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let display_group_list = ServerRspMsg::DisplayGroupList {
            req_id: decode_i32(&mut fields_itr)?,
            groups: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(display_group_list).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_display_group_updated(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let display_group_updated = ServerRspMsg::DisplayGroupUpdated {
            req_id: decode_i32(&mut fields_itr)?,
            contract_info: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(display_group_updated).unwrap();

        Ok(())
    }
    fn process_error_message(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let error = ServerRspMsg::ErrMsg {
            req_id: decode_i32(&mut fields_itr)?,
            error_code: decode_i32(&mut fields_itr)?,
            error_str: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(error).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_execution_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let mut version = self.server_version;

        if self.server_version < MIN_SERVER_VER_LAST_LIQUIDITY {
            version = decode_i32(&mut fields_itr)?;
        }

        let mut req_id = -1;

        if version >= 7 {
            req_id = decode_i32(&mut fields_itr)?;
        }

        let order_id = decode_i32(&mut fields_itr)?;

        // decode contract fields
        let mut contract = Contract::default();
        contract.con_id = decode_i32(&mut fields_itr)?; // ver 5 field
        contract.symbol = decode_string(&mut fields_itr)?;
        contract.sec_type = decode_string(&mut fields_itr)?;
        contract.last_trade_date_or_contract_month = decode_string(&mut fields_itr)?;
        contract.strike = decode_f64(&mut fields_itr)?;
        contract.right = decode_string(&mut fields_itr)?;
        if version >= 9 {
            contract.multiplier = decode_string(&mut fields_itr)?;
        }
        contract.exchange = decode_string(&mut fields_itr)?;
        contract.currency = decode_string(&mut fields_itr)?;
        contract.local_symbol = decode_string(&mut fields_itr)?;
        if version >= 10 {
            contract.trading_class = decode_string(&mut fields_itr)?;
        }

        // decode execution fields
        let mut execution = Execution::default();
        execution.order_id = order_id;
        execution.exec_id = decode_string(&mut fields_itr)?;
        execution.time = decode_string(&mut fields_itr)?;
        execution.acct_number = decode_string(&mut fields_itr)?;
        execution.exchange = decode_string(&mut fields_itr)?;
        execution.side = decode_string(&mut fields_itr)?;

        if self.server_version >= MIN_SERVER_VER_FRACTIONAL_POSITIONS {
            execution.shares = decode_f64(&mut fields_itr)?;
        } else {
            execution.shares = decode_i32(&mut fields_itr)? as f64;
        }

        execution.price = decode_f64(&mut fields_itr)?;
        execution.perm_id = decode_i32(&mut fields_itr)?; // ver 2 field
        execution.client_id = decode_i32(&mut fields_itr)?; // ver 3 field
        execution.liquidation = decode_i32(&mut fields_itr)?; // ver 4 field

        if version >= 6 {
            execution.cum_qty = decode_f64(&mut fields_itr)?;
            execution.avg_price = decode_f64(&mut fields_itr)?;
        }

        if version >= 8 {
            execution.order_ref = decode_string(&mut fields_itr)?;
        }

        if version >= 9 {
            execution.ev_rule = decode_string(&mut fields_itr)?;

            let tmp_ev_mult = (&mut fields_itr).peekable().peek().unwrap().as_str();
            if tmp_ev_mult != "" {
                execution.ev_multiplier = decode_f64(&mut fields_itr)?;
            } else {
                execution.ev_multiplier = 1.0;
            }
        }

        if self.server_version >= MIN_SERVER_VER_MODELS_SUPPORT {
            execution.model_code = decode_string(&mut fields_itr)?;
        }
        if self.server_version >= MIN_SERVER_VER_LAST_LIQUIDITY {
            execution.last_liquidity = decode_i32(&mut fields_itr)?;
        }

        let exec_details = ServerRspMsg::ExecutionData {
            req_id: req_id,
            contract: contract.clone(),
            execution: execution.clone(),
        };

        self.send_queue.send(exec_details).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_execution_data_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let exec_details_end = ServerRspMsg::ExecutionDataEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(exec_details_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_family_codes(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let family_codes_count = decode_i32(&mut fields_itr)?;
        let mut family_codes: Vec<FamilyCode> = vec![];
        for _ in 0..family_codes_count {
            let mut fam_code = FamilyCode::default();
            fam_code.account_id = decode_string(&mut fields_itr)?;
            fam_code.family_code_str = decode_string(&mut fields_itr)?;
            family_codes.push(fam_code);
        }

        let family_codes_msg = ServerRspMsg::FamilyCodes {
            family_codes: family_codes.clone(),
        };

        self.send_queue.send(family_codes_msg).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_fundamental_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let fundamental_data = ServerRspMsg::FundamentalData {
            req_id: decode_i32(&mut fields_itr)?,
            data: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(fundamental_data).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_head_timestamp(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let fundamental_data = ServerRspMsg::FundamentalData {
            req_id: decode_i32(&mut fields_itr)?,
            data: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(fundamental_data).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_histogram_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let num_points = decode_i32(&mut fields_itr)?;

        let mut histogram = vec![];
        for _ in 0..num_points {
            let mut data_point = HistogramData::default();
            data_point.price = decode_f64(&mut fields_itr)?;
            data_point.count = decode_i32(&mut fields_itr)?;
            histogram.push(data_point);
        }

        let histogram_data = ServerRspMsg::HistogramData {
            req_id: req_id,
            items: histogram,
        };

        self.send_queue.send(histogram_data).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //throw away message_id
        fields_itr.next();

        if self.server_version < MIN_SERVER_VER_SYNT_REALTIME_BARS {
            fields_itr.next();
        }

        let req_id = decode_i32(&mut fields_itr)?;
        let start_date = decode_string(&mut fields_itr)?; // ver 2 field
        let end_date = decode_string(&mut fields_itr)?; // ver 2 field

        let _peek = *(fields_itr.clone()).peekable().peek().unwrap();

        let bar_count = decode_i32(&mut fields_itr)?;

        for _ in 0..bar_count {
            let mut bar = BarData::default();
            bar.date = decode_string(&mut fields_itr)?;
            bar.open = decode_f64(&mut fields_itr)?;
            bar.high = decode_f64(&mut fields_itr)?;
            bar.low = decode_f64(&mut fields_itr)?;
            bar.close = decode_f64(&mut fields_itr)?;
            bar.volume = if self.server_version < MIN_SERVER_VER_SYNT_REALTIME_BARS {
                decode_i32(&mut fields_itr)? as i64
            } else {
                decode_i64(&mut fields_itr)?
            };
            bar.average = decode_f64(&mut fields_itr)?;

            if self.server_version < MIN_SERVER_VER_SYNT_REALTIME_BARS {
                decode_string(&mut fields_itr)?; //has_gaps
            }

            bar.bar_count = decode_i32(&mut fields_itr)?; // ver 3 field

            let historical_data_msg = ServerRspMsg::HistoricalData {
                req_id: req_id,
                bar: bar.clone(),
            };

            self.send_queue.send(historical_data_msg).unwrap();
        }

        let historical_data_end = ServerRspMsg::HistoricalDataEnd {
            req_id: req_id,
            start: start_date.clone(),
            end: end_date.clone(),
        };

        // send end of dataset marker
        self.send_queue.send(historical_data_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_data_update(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let mut bar = BarData::default();
        bar.bar_count = decode_i32(&mut fields_itr)?;
        bar.date = decode_string(&mut fields_itr)?;
        bar.open = decode_f64(&mut fields_itr)?;
        bar.close = decode_f64(&mut fields_itr)?;
        bar.high = decode_f64(&mut fields_itr)?;
        bar.low = decode_f64(&mut fields_itr)?;
        bar.average = decode_f64(&mut fields_itr)?;
        bar.volume = decode_i64(&mut fields_itr)?;

        let historical_data_update = ServerRspMsg::HistoricalDataUpdate {
            req_id: req_id,
            bar: bar.clone(),
        };

        self.send_queue.send(historical_data_update).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_news(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let historical_news = ServerRspMsg::HistoricalNews {
            req_id: decode_i32(&mut fields_itr)?,
            time: decode_string(&mut fields_itr)?,
            provider_code: decode_string(&mut fields_itr)?,
            article_id: decode_string(&mut fields_itr)?,
            headline: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(historical_news).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_news_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let historical_news_end = ServerRspMsg::HistoricalNewsEnd {
            req_id: decode_i32(&mut fields_itr)?,
            has_more: decode_bool(&mut fields_itr)?,
        };

        // send end of dataset marker
        self.send_queue.send(historical_news_end).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_ticks(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let tick_count = decode_i32(&mut fields_itr)?;

        let mut ticks = vec![];

        for _ in 0..tick_count {
            let mut historical_tick = HistoricalTick::default();
            historical_tick.time = decode_i32(&mut fields_itr)?;
            fields_itr.next(); // for consistency
            historical_tick.price = decode_f64(&mut fields_itr)?;
            historical_tick.size = decode_i32(&mut fields_itr)?;
            ticks.push(historical_tick);
        }

        let done = decode_bool(&mut fields_itr)?;

        let historical_ticks = ServerRspMsg::HistoricalTicks {
            req_id: req_id,
            ticks: ticks.clone(),
            done: done,
        };

        self.send_queue.send(historical_ticks).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_ticks_bid_ask(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let tick_count = decode_i32(&mut fields_itr)?;

        let mut ticks = vec![];

        for _ in 0..tick_count {
            let mut historical_tick_bid_ask = HistoricalTickBidAsk::default();
            historical_tick_bid_ask.time = decode_i32(&mut fields_itr)?;
            let mask = decode_i32(&mut fields_itr)?;
            let mut tick_attrib_bid_ask = TickAttribBidAsk::default();
            tick_attrib_bid_ask.ask_past_high = mask & 1 != 0;
            tick_attrib_bid_ask.bid_past_low = mask & 2 != 0;
            historical_tick_bid_ask.tick_attrib_bid_ask = tick_attrib_bid_ask;
            historical_tick_bid_ask.price_bid = decode_f64(&mut fields_itr)?;
            historical_tick_bid_ask.price_ask = decode_f64(&mut fields_itr)?;
            historical_tick_bid_ask.size_bid = decode_i32(&mut fields_itr)?;
            historical_tick_bid_ask.size_ask = decode_i32(&mut fields_itr)?;
            ticks.push(historical_tick_bid_ask);
        }

        let done = decode_bool(&mut fields_itr)?;

        let historical_ticks_bid_ask = ServerRspMsg::HistoricalTicksBidAsk {
            req_id: req_id,
            ticks: ticks.clone(),
            done: done,
        };

        self.send_queue.send(historical_ticks_bid_ask).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_historical_ticks_last(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let tick_count = decode_i32(&mut fields_itr)?;

        let mut ticks = vec![];

        for _ in 0..tick_count {
            let mut historical_tick_last = HistoricalTickLast::default();
            historical_tick_last.time = decode_i32(&mut fields_itr)?;
            let mask = decode_i32(&mut fields_itr)?;
            let mut tick_attrib_last = TickAttribLast::default();
            tick_attrib_last.past_limit = mask & 1 != 0;
            tick_attrib_last.unreported = mask & 2 != 0;
            historical_tick_last.tick_attrib_last = tick_attrib_last;
            historical_tick_last.price = decode_f64(&mut fields_itr)?;
            historical_tick_last.size = decode_i32(&mut fields_itr)?;
            historical_tick_last.exchange = decode_string(&mut fields_itr)?;
            historical_tick_last.special_conditions = decode_string(&mut fields_itr)?;
            ticks.push(historical_tick_last);
        }

        let done = decode_bool(&mut fields_itr)?;

        let historical_ticks_last_msg = ServerRspMsg::HistoricalTicksLast {
            req_id: req_id,
            ticks: ticks.clone(),
            done: done,
        };

        self.send_queue.send(historical_ticks_last_msg).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_managed_accounts(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        info!("calling managed_accounts");
        let managed_accounts = ServerRspMsg::ManagedAccts {
            accounts_list: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(managed_accounts).unwrap();

        info!("finished calling managed_accounts");
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_market_data_type(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let marketdatatype = ServerRspMsg::MarketDataType {
            req_id: decode_i32(&mut fields_itr)?,
            market_data_type: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(marketdatatype).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_market_depth(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let update_mkt_depth = ServerRspMsg::MarketDepth {
            req_id: decode_i32(&mut fields_itr)?,
            position: decode_i32(&mut fields_itr)?,
            operation: decode_i32(&mut fields_itr)?,
            side: decode_i32(&mut fields_itr)?,
            price: decode_f64(&mut fields_itr)?,
            size: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(update_mkt_depth).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_market_depth_l2(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let position = decode_i32(&mut fields_itr)?;
        let market_maker = decode_string(&mut fields_itr)?;
        let operation = decode_i32(&mut fields_itr)?;
        let side = decode_i32(&mut fields_itr)?;
        let price = decode_f64(&mut fields_itr)?;
        let size = decode_i32(&mut fields_itr)?;
        let mut is_smart_depth = false;

        if self.server_version >= MIN_SERVER_VER_SMART_DEPTH {
            is_smart_depth = decode_bool(&mut fields_itr)?;
        }

        let update_mkt_depth_l2 = ServerRspMsg::MarketDepthL2 {
            req_id: req_id,
            position: position,
            market_maker: market_maker,
            operation: operation,
            side: side,
            price: price,
            size: size,
            is_smart_depth: is_smart_depth,
        };

        self.send_queue.send(update_mkt_depth_l2).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_market_rule(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let market_rule_id = decode_i32(&mut fields_itr)?;

        let price_increments_count = decode_i32(&mut fields_itr)?;
        let mut price_increments = vec![];

        for _ in 0..price_increments_count {
            let mut prc_inc = PriceIncrement::default();
            prc_inc.low_edge = decode_f64(&mut fields_itr)?;
            prc_inc.increment = decode_f64(&mut fields_itr)?;
            price_increments.push(prc_inc);
        }

        let market_rule = ServerRspMsg::MarketRule {
            market_rule_id: market_rule_id,
            price_increments: price_increments,
        };

        self.send_queue.send(market_rule).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_market_depth_exchanges(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let mut depth_mkt_data_descriptions = vec![];
        let depth_mkt_data_descriptions_count = decode_i32(&mut fields_itr)?;

        for _ in 0..depth_mkt_data_descriptions_count {
            let mut desc = DepthMktDataDescription::default();
            desc.exchange = decode_string(&mut fields_itr)?;
            desc.sec_type = decode_string(&mut fields_itr)?;
            if self.server_version >= MIN_SERVER_VER_SERVICE_DATA_TYPE {
                desc.listing_exch = decode_string(&mut fields_itr)?;
                desc.service_data_type = decode_string(&mut fields_itr)?;
                desc.agg_group = decode_i32(&mut fields_itr)?;
            } else {
                decode_i32(&mut fields_itr)?; // boolean notSuppIsL2
            }
            depth_mkt_data_descriptions.push(desc);
        }

        let market_depth_xchng = ServerRspMsg::MktDepthExchanges {
            depth_mkt_data_descriptions: depth_mkt_data_descriptions,
        };

        self.send_queue.send(market_depth_xchng).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_news_article(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let news_article = ServerRspMsg::NewsArticle {
            req_id: decode_i32(&mut fields_itr)?,
            article_type: decode_i32(&mut fields_itr)?,
            article_text: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(news_article).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_news_bulletins(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let news_bulletin = ServerRspMsg::NewsBulletins {
            msg_id: decode_i32(&mut fields_itr)?,
            msg_type: decode_i32(&mut fields_itr)?,
            news_message: decode_string(&mut fields_itr)?,
            origin_exch: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(news_bulletin).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_news_providers(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let mut news_providers = vec![];
        let news_providers_count = decode_i32(&mut fields_itr)?;
        for _ in 0..news_providers_count {
            let mut provider = NewsProvider::default();
            provider.code = decode_string(&mut fields_itr)?;
            provider.name = decode_string(&mut fields_itr)?;
            news_providers.push(provider);
        }

        let news_providers = ServerRspMsg::NewsProviders {
            news_providers: news_providers,
        };

        self.send_queue.send(news_providers).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_next_valid_id(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let next_valid_id = ServerRspMsg::NextValidId {
            order_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(next_valid_id).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_open_order(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //info!("Processing open order");
        //throw away message_id
        fields_itr.next();

        let mut order = Order::default();
        let mut contract = Contract::default();
        let mut order_state = OrderState::default();

        let mut version = self.server_version;
        if self.server_version < MIN_SERVER_VER_ORDER_CONTAINER {
            version = decode_i32(&mut fields_itr)?;
        }

        let mut order_decoder = OrderDecoder::new(
            &mut contract,
            &mut order,
            &mut order_state,
            version,
            self.server_version,
        );

        order_decoder.decode_open(&mut fields_itr)?;
        let open_order_msg = ServerRspMsg::OpenOrder {
            order_id: order.order_id,
            contract: contract,
            order: order,
            order_state: order_state,
        };

        self.send_queue.send(open_order_msg).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_order_bound(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let order_bound = ServerRspMsg::OrderBound {
            req_id: decode_i32(&mut fields_itr)?,
            api_client_id: decode_i32(&mut fields_itr)?,
            api_order_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(order_bound).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_order_status(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        if self.server_version < MIN_SERVER_VER_MARKET_CAP_PRICE {
            fields_itr.next();
        }

        let order_id = decode_i32(&mut fields_itr)?;

        let status = decode_string(&mut fields_itr)?;

        let filled;
        if self.server_version >= MIN_SERVER_VER_FRACTIONAL_POSITIONS {
            filled = decode_f64(&mut fields_itr)?;
        } else {
            filled = decode_i32(&mut fields_itr)? as f64;
        }

        let remaining;

        if self.server_version >= MIN_SERVER_VER_FRACTIONAL_POSITIONS {
            remaining = decode_f64(&mut fields_itr)?;
        } else {
            remaining = decode_i32(&mut fields_itr)? as f64;
        }

        let avg_fill_price = decode_f64(&mut fields_itr)?;

        let perm_id = decode_i32(&mut fields_itr)?; // ver 2 field
        let parent_id = decode_i32(&mut fields_itr)?; // ver 3 field
        let last_fill_price = decode_f64(&mut fields_itr)?; // ver 4 field
        let client_id = decode_i32(&mut fields_itr)?; // ver 5 field
        let why_held = decode_string(&mut fields_itr)?; // ver 6 field

        let mut mkt_cap_price = 0.0;
        if self.server_version >= MIN_SERVER_VER_MARKET_CAP_PRICE {
            mkt_cap_price = decode_f64(&mut fields_itr)?;
        }
        let order_status = ServerRspMsg::OrderStatus {
            order_id,
            status,
            filled,
            remaining,
            avg_fill_price,
            perm_id,
            parent_id,
            last_fill_price,
            client_id,
            why_held,
            mkt_cap_price,
        };

        self.send_queue.send(order_status).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_pnl(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let daily_pnl = decode_f64(&mut fields_itr)?;
        let mut unrealized_pnl = 0.0;
        let mut realized_pnl = 0.0;

        if self.server_version >= MIN_SERVER_VER_UNREALIZED_PNL {
            unrealized_pnl = decode_f64(&mut fields_itr)?;
        }

        if self.server_version >= MIN_SERVER_VER_REALIZED_PNL {
            realized_pnl = decode_f64(&mut fields_itr)?;
        }

        let pnl_msg = ServerRspMsg::Pnl {
            req_id,
            daily_pnl,
            unrealized_pnl,
            realized_pnl,
        };

        self.send_queue.send(pnl_msg).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_pnl_single(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let pos = decode_i32(&mut fields_itr)?;
        let daily_pnl = decode_f64(&mut fields_itr)?;
        let mut unrealized_pnl = 0.0;
        let mut realized_pnl = 0.0;

        if self.server_version >= MIN_SERVER_VER_UNREALIZED_PNL {
            unrealized_pnl = decode_f64(&mut fields_itr)?;
        }

        if self.server_version >= MIN_SERVER_VER_REALIZED_PNL {
            realized_pnl = decode_f64(&mut fields_itr)?;
        }

        let value = decode_f64(&mut fields_itr)?;
        let pnl_single = ServerRspMsg::PnlSingle {
            req_id,
            pos,
            daily_pnl,
            unrealized_pnl,
            realized_pnl,
            value,
        };

        self.send_queue.send(pnl_single).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_portfolio_value(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let version = decode_i32(&mut fields_itr)?;

        // read contract fields
        let mut contract = Contract::default();
        contract.con_id = decode_i32(&mut fields_itr)?; // ver 6 field
        contract.symbol = decode_string(&mut fields_itr)?;
        contract.sec_type = decode_string(&mut fields_itr)?;
        contract.last_trade_date_or_contract_month = decode_string(&mut fields_itr)?;
        contract.strike = decode_f64(&mut fields_itr)?;
        contract.right = decode_string(&mut fields_itr)?;

        if version >= 7 {
            contract.multiplier = decode_string(&mut fields_itr)?;
            contract.primary_exchange = decode_string(&mut fields_itr)?;
        }

        contract.currency = decode_string(&mut fields_itr)?;
        contract.local_symbol = decode_string(&mut fields_itr)?; // ver 2 field
        if version >= 8 {
            contract.trading_class = decode_string(&mut fields_itr)?;
        }

        let position;
        if self.server_version >= MIN_SERVER_VER_FRACTIONAL_POSITIONS {
            position = decode_f64(&mut fields_itr)?;
        } else {
            position = decode_i32(&mut fields_itr)? as f64;
        }

        let market_price = decode_f64(&mut fields_itr)?;
        let market_value = decode_f64(&mut fields_itr)?;
        let average_cost = decode_f64(&mut fields_itr)?; // ver 3 field
        let unrealized_pnl = decode_f64(&mut fields_itr)?; // ver 3 field
        let realized_pnl = decode_f64(&mut fields_itr)?; // ver 3 field

        let account_name = decode_string(&mut fields_itr)?; // ver 4 field

        if version == 6 && self.server_version == 39 {
            contract.primary_exchange = decode_string(&mut fields_itr)?;
        }

        let update_portfolio = ServerRspMsg::PortfolioValue {
            contract,
            position,
            market_price,
            market_value,
            average_cost,
            unrealized_pnl,
            realized_pnl,
            account_name,
        };

        self.send_queue.send(update_portfolio).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_position_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let version = decode_i32(&mut fields_itr)?;

        let account = decode_string(&mut fields_itr)?;

        // decode contract fields
        let mut contract = Contract::default();
        contract.con_id = decode_i32(&mut fields_itr)?;
        contract.symbol = decode_string(&mut fields_itr)?;
        contract.sec_type = decode_string(&mut fields_itr)?;
        contract.last_trade_date_or_contract_month = decode_string(&mut fields_itr)?;
        contract.strike = decode_f64(&mut fields_itr)?;
        contract.right = decode_string(&mut fields_itr)?;
        contract.multiplier = decode_string(&mut fields_itr)?;
        contract.exchange = decode_string(&mut fields_itr)?;
        contract.currency = decode_string(&mut fields_itr)?;
        contract.local_symbol = decode_string(&mut fields_itr)?;
        if version >= 2 {
            contract.trading_class = decode_string(&mut fields_itr)?;
        }

        let position;
        if self.server_version >= MIN_SERVER_VER_FRACTIONAL_POSITIONS {
            position = decode_f64(&mut fields_itr)?;
        } else {
            position = decode_i32(&mut fields_itr)? as f64;
        }

        let mut avg_cost = 0.0;
        if version >= 3 {
            avg_cost = decode_f64(&mut fields_itr)?;
        }

        let position_data = ServerRspMsg::PositionData {
            account,
            contract,
            position,
            avg_cost,
        };

        self.send_queue.send(position_data).unwrap();

        Ok(())
    }

    fn process_end_msg_noarg(&mut self, cmd: ServerRspMsg) -> Result<(), IBKRApiLibError> {
        self.send_queue.send(cmd).unwrap();
        Ok(())
    }

    fn process_position_multi(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let account = decode_string(&mut fields_itr)?;

        // decode contract fields
        let mut contract = Contract::default();
        contract.con_id = decode_i32(&mut fields_itr)?;
        contract.symbol = decode_string(&mut fields_itr)?;
        contract.sec_type = decode_string(&mut fields_itr)?;
        contract.last_trade_date_or_contract_month = decode_string(&mut fields_itr)?;
        contract.strike = decode_f64(&mut fields_itr)?;
        contract.right = decode_string(&mut fields_itr)?;
        contract.multiplier = decode_string(&mut fields_itr)?;
        contract.exchange = decode_string(&mut fields_itr)?;
        contract.currency = decode_string(&mut fields_itr)?;
        contract.local_symbol = decode_string(&mut fields_itr)?;
        contract.trading_class = decode_string(&mut fields_itr)?;

        let position = decode_f64(&mut fields_itr)?;
        let avg_cost = decode_f64(&mut fields_itr)?;
        let model_code = decode_string(&mut fields_itr)?;

        let position_multi = ServerRspMsg::PositionMulti {
            req_id,
            account,
            model_code,
            contract,
            position,
            avg_cost,
        };

        self.send_queue.send(position_multi).unwrap();

        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_position_multi_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let position_multi_end = ServerRspMsg::PositionMultiEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(position_multi_end).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_real_time_bars(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let mut bar = RealTimeBar::default();
        bar.date_time = decode_string(&mut fields_itr)?;
        bar.open = decode_f64(&mut fields_itr)?;
        bar.high = decode_f64(&mut fields_itr)?;
        bar.low = decode_f64(&mut fields_itr)?;
        bar.close = decode_f64(&mut fields_itr)?;
        bar.volume = decode_i64(&mut fields_itr)?;
        bar.wap = decode_f64(&mut fields_itr)?;
        bar.count = decode_i32(&mut fields_itr)?;

        let real_time_bars = ServerRspMsg::RealTimeBars { req_id, bar: bar };

        self.send_queue.send(real_time_bars).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_receive_fa(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let fa_data_type = decode_i32(&mut fields_itr)?;
        let xml = decode_string(&mut fields_itr)?;

        let receive_fa = ServerRspMsg::ReceiveFa {
            fa_data: FromPrimitive::from_i32(fa_data_type).unwrap(),
            cxml: xml,
        };

        self.send_queue.send(receive_fa).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_reroute_mkt_data_req(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let reroute_mkt_data = ServerRspMsg::RerouteMktDataReq {
            req_id: decode_i32(&mut fields_itr)?,
            con_id: decode_i32(&mut fields_itr)?,
            exchange: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(reroute_mkt_data).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_reroute_mkt_depth_req(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let reroute_mkt_depth = ServerRspMsg::RerouteMktDepthReq {
            req_id: decode_i32(&mut fields_itr)?,
            con_id: decode_i32(&mut fields_itr)?,
            exchange: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(reroute_mkt_depth).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_scanner_data(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let number_of_elements = decode_i32(&mut fields_itr)?;

        for _ in 0..number_of_elements {
            let mut data = ScanData::default();
            data.contract = ContractDetails::default();

            data.rank = decode_i32(&mut fields_itr)?;
            data.contract.contract.con_id = decode_i32(&mut fields_itr)?; // ver 3 field
            data.contract.contract.symbol = decode_string(&mut fields_itr)?;
            data.contract.contract.sec_type = decode_string(&mut fields_itr)?;
            data.contract.contract.last_trade_date_or_contract_month =
                decode_string(&mut fields_itr)?;
            data.contract.contract.strike = decode_f64(&mut fields_itr)?;
            data.contract.contract.right = decode_string(&mut fields_itr)?;
            data.contract.contract.exchange = decode_string(&mut fields_itr)?;
            data.contract.contract.currency = decode_string(&mut fields_itr)?;
            data.contract.contract.local_symbol = decode_string(&mut fields_itr)?;
            data.contract.market_name = decode_string(&mut fields_itr)?;
            data.contract.contract.trading_class = decode_string(&mut fields_itr)?;
            data.distance = decode_string(&mut fields_itr)?;
            data.benchmark = decode_string(&mut fields_itr)?;
            data.projection = decode_string(&mut fields_itr)?;
            data.legs = decode_string(&mut fields_itr)?;
            let scanner_data = ServerRspMsg::ScannerData {
                req_id,
                rank: data.rank,
                contract_details: data.contract,
                distance: data.distance,
                benchmark: data.benchmark,
                projection: data.projection,
                legs_str: data.legs,
            };

            self.send_queue.send(scanner_data).unwrap();
        }

        let scanner_data_end = ServerRspMsg::ScannerDataEnd { req_id };

        self.send_queue.send(scanner_data_end).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_scanner_parameters(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let xml = decode_string(&mut fields_itr)?;
        let scanner_params = ServerRspMsg::ScannerParameters { xml };

        self.send_queue.send(scanner_params).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_security_definition_option_parameter(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let exchange = decode_string(&mut fields_itr)?;
        let underlying_con_id = decode_i32(&mut fields_itr)?;
        let trading_class = decode_string(&mut fields_itr)?;
        let multiplier = decode_string(&mut fields_itr)?;

        let exp_count = decode_i32(&mut fields_itr)?;
        let mut expirations = HashSet::new();
        for _ in 0..exp_count {
            let expiration = decode_string(&mut fields_itr)?;
            expirations.insert(expiration);
        }

        let strike_count = decode_i32(&mut fields_itr)?;
        let mut strikes = HashSet::new();
        for _ in 0..strike_count {
            let strike = decode_f64(&mut fields_itr)?;
            let big_strike = Decimal::from_f64(strike).unwrap();
            strikes.insert(big_strike);
        }
        let security_def_opt_param = ServerRspMsg::SecurityDefinitionOptionParameter {
            req_id,
            exchange,
            underlying_con_id,
            trading_class,
            multiplier,
            expirations,
            strikes,
        };

        self.send_queue.send(security_def_opt_param).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_security_definition_option_parameter_end(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let security_def_opt_param_end = ServerRspMsg::SecurityDefinitionOptionParameterEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(security_def_opt_param_end).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_smart_components(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let count = decode_i32(&mut fields_itr)?;

        let mut smart_components = vec![];
        for _ in 0..count {
            let mut smart_component = SmartComponent::default();
            smart_component.bit_number = decode_i32(&mut fields_itr)?;
            smart_component.exchange = decode_string(&mut fields_itr)?;
            smart_component.exchange_letter = decode_string(&mut fields_itr)?;
            smart_components.push(smart_component)
        }
        let smart_component_msg = ServerRspMsg::SmartComponents {
            req_id,
            smart_components,
        };

        self.send_queue.send(smart_component_msg).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_soft_dollar_tiers(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let count = decode_i32(&mut fields_itr)?;

        let mut tiers = vec![];
        for _ in 0..count {
            let mut tier = SoftDollarTier::default();
            tier.name = decode_string(&mut fields_itr)?;
            tier.val = decode_string(&mut fields_itr)?;
            tier.display_name = decode_string(&mut fields_itr)?;
            tiers.push(tier);
        }

        let soft_dollar_tiers = ServerRspMsg::SoftDollarTiers { req_id, tiers };

        self.send_queue.send(soft_dollar_tiers).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_symbol_samples(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;

        let count = decode_i32(&mut fields_itr)?;
        let mut contract_descriptions = vec![];
        for _ in 0..count {
            let mut con_desc = ContractDescription::default();
            con_desc.contract.con_id = decode_i32(&mut fields_itr)?;
            con_desc.contract.symbol = decode_string(&mut fields_itr)?;
            con_desc.contract.sec_type = decode_string(&mut fields_itr)?;
            con_desc.contract.primary_exchange = decode_string(&mut fields_itr)?;
            con_desc.contract.currency = decode_string(&mut fields_itr)?;

            let derivative_sec_types_cnt = decode_i32(&mut fields_itr)?;
            con_desc.derivative_sec_types = vec![];
            for _ in 0..derivative_sec_types_cnt {
                let deriv_sec_type = decode_string(&mut fields_itr)?;
                con_desc.derivative_sec_types.push(deriv_sec_type);
            }
            contract_descriptions.push(con_desc)
        }

        let symbol_samples = ServerRspMsg::SymbolSamples {
            req_id,
            contract_descriptions,
        };

        self.send_queue.send(symbol_samples).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_by_tick(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let req_id = decode_i32(&mut fields_itr)?;
        let tick_type = decode_i32(&mut fields_itr)?;
        let time = decode_i64(&mut fields_itr)?;

        let tick_msg = match tick_type {
            0 => return Ok(()), // None
            1..=2 =>
            // Last (1) or AllLast (2)
            {
                let price = decode_f64(&mut fields_itr)?;
                let size = decode_i32(&mut fields_itr)?;
                let mask = decode_i32(&mut fields_itr)?;
                let mut tick_attrib_last = TickAttribLast::default();
                tick_attrib_last.past_limit = mask & 1 != 0;
                tick_attrib_last.unreported = mask & 2 != 0;
                let exchange = decode_string(&mut fields_itr)?;
                let special_conditions = decode_string(&mut fields_itr)?;

                TickMsgType::AllLast {
                    price,
                    size,
                    tick_attrib_last,
                    exchange,
                    special_conditions,
                }
            }
            3 =>
            // BidAsk
            {
                let bid_price = decode_f64(&mut fields_itr)?;
                let ask_price = decode_f64(&mut fields_itr)?;
                let bid_size = decode_i32(&mut fields_itr)?;
                let ask_size = decode_i32(&mut fields_itr)?;
                let mask = decode_i32(&mut fields_itr)?;
                let mut tick_attrib_bid_ask = TickAttribBidAsk::default();
                tick_attrib_bid_ask.bid_past_low = mask & 1 != 0;
                tick_attrib_bid_ask.ask_past_high = mask & 2 != 0;

                TickMsgType::BidAsk {
                    bid_price,
                    ask_price,
                    bid_size,
                    ask_size,
                    tick_attrib_bid_ask,
                }
            }
            4 =>
            // MidPoint
            {
                let mid_point = decode_f64(&mut fields_itr)?;

                TickMsgType::MidPoint { mid_point }
            }
            _ => return Ok(()),
        };
        let tick_by_tick_msg = ServerRspMsg::TickByTick {
            req_id,
            tick_type,
            time,
            tick_msg,
        };
        self.send_queue.send(tick_by_tick_msg).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    #[allow(dead_code)]
    fn process_tick_efp(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let ticker_id = decode_i32(&mut fields_itr)?;
        let tick_type = FromPrimitive::from_i32(decode_i32(&mut fields_itr)?).unwrap();
        let basis_points = decode_f64(&mut fields_itr)?;
        let formatted_basis_points = decode_string(&mut fields_itr)?;
        let implied_futures_price = decode_f64(&mut fields_itr)?;
        let hold_days = decode_i32(&mut fields_itr)?;
        let future_last_trade_date = decode_string(&mut fields_itr)?;
        let dividend_impact = decode_f64(&mut fields_itr)?;
        let dividends_to_last_trade_date = decode_f64(&mut fields_itr)?;

        let tick_efp = ServerRspMsg::TickEfp {
            ticker_id,
            tick_type,
            basis_points,
            formatted_basis_points,
            implied_futures_price,
            hold_days,
            future_last_trade_date,
            dividend_impact,
            dividends_to_last_trade_date,
        };

        self.send_queue.send(tick_efp).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_generic(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let ticker_id = decode_i32(&mut fields_itr)?;
        let tick_type = FromPrimitive::from_i32(decode_i32(&mut fields_itr)?).unwrap();
        let value = decode_f64(&mut fields_itr)?;

        let tick_generic = ServerRspMsg::TickGeneric {
            ticker_id,
            tick_type,
            value,
        };

        self.send_queue.send(tick_generic).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_news(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let tick_news = ServerRspMsg::TickNews {
            ticker_id: decode_i32(&mut fields_itr)?,
            time_stamp: decode_i32(&mut fields_itr)?,
            provider_code: decode_string(&mut fields_itr)?,
            article_id: decode_string(&mut fields_itr)?,
            headline: decode_string(&mut fields_itr)?,
            extra_data: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(tick_news).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_option_computation(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let version = decode_i32(&mut fields_itr)?;
        let ticker_id = decode_i32(&mut fields_itr)?;
        let tick_type = FromPrimitive::from_i32(decode_i32(&mut fields_itr)?).unwrap();
        let mut implied_vol = decode_f64(&mut fields_itr)?;
        if approx_eq!(f64, implied_vol, -1.0, ulps = 2) {
            // -1 is the "not yet computed" indicator
            implied_vol = f64::max_value();
        }

        let mut delta = decode_f64(&mut fields_itr)?;
        if approx_eq!(f64, delta, -2.0, ulps = 2) {
            // -2 is the "not yet computed" indicator
            delta = f64::max_value();
        }
        let mut opt_price = f64::max_value();
        let mut pv_dividend = f64::max_value();
        let mut gamma = f64::max_value();
        let mut vega = f64::max_value();
        let mut theta = f64::max_value();
        let mut und_price = f64::max_value();
        if version >= 6
            || matches!(
                tick_type,
                TickType::ModelOption | TickType::DelayedModelOption
            )
        {
            // introduced in version == 5
            opt_price = decode_f64(&mut fields_itr)?;
            if approx_eq!(f64, opt_price, -1.0, ulps = 2) {
                // -1 is the "not yet computed" indicator
                opt_price = f64::max_value();
            }
            pv_dividend = decode_f64(&mut fields_itr)?;
            if approx_eq!(f64, pv_dividend, -1.0, ulps = 2) {
                // -1 is the "not yet computed" indicator
                pv_dividend = f64::max_value();
            }
        }
        if version >= 6 {
            gamma = decode_f64(&mut fields_itr)?;
            if approx_eq!(f64, gamma, -2.0, ulps = 2) {
                // -2 is the "not yet computed" indicator
                gamma = f64::max_value();
            }
            vega = decode_f64(&mut fields_itr)?;
            if approx_eq!(f64, vega, -2.0, ulps = 2) {
                // -2 is the "not yet computed" indicator
                vega = f64::max_value();
            }
            theta = decode_f64(&mut fields_itr)?;
            if approx_eq!(f64, theta, -2.0, ulps = 2) {
                // -2 is the "not yet computed" indicator
                theta = f64::max_value();
            }
            und_price = decode_f64(&mut fields_itr)?;
            if approx_eq!(f64, und_price, -1.0, ulps = 2) {
                // -1 is the "not yet computed" indicator
                und_price = f64::max_value();
            }
        }

        let tick_option_computation = ServerRspMsg::TickOptionComputation {
            ticker_id,
            tick_type,
            implied_vol,
            delta,
            opt_price,
            pv_dividend,
            gamma,
            vega,
            theta,
            und_price,
        };

        self.send_queue.send(tick_option_computation).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_req_params(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();

        let tick_req_params = ServerRspMsg::TickReqParams {
            ticker_id: decode_i32(&mut fields_itr)?,
            min_tick: decode_f64(&mut fields_itr)?,
            bbo_exchange: decode_string(&mut fields_itr)?,
            snapshot_permissions: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(tick_req_params).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_size(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let tick_size = ServerRspMsg::TickSize {
            req_id: decode_i32(&mut fields_itr)?,
            tick_type: FromPrimitive::from_i32(decode_i32(&mut fields_itr)?).unwrap(),
            size: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(tick_size).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_tick_snapshot_end(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let tick_snapshot_end = ServerRspMsg::TickSnapshotEnd {
            req_id: decode_i32(&mut fields_itr)?,
        };

        self.send_queue.send(tick_snapshot_end).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_verify_and_auth_completed(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();
        let _is_successful_str = decode_string(&mut fields_itr)?;
        let is_successful = "true" == decode_string(&mut fields_itr)?;
        let error_text = decode_string(&mut fields_itr)?;

        let verify_and_auth = ServerRspMsg::VerifyAndAuthCompleted {
            is_successful,
            error_text,
        };

        self.send_queue.send(verify_and_auth).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_verify_and_auth_message_api(
        &mut self,
        fields: &[String],
    ) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();

        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let verify_and_auth_message = ServerRspMsg::VerifyAndAuthMessageApi {
            api_data: decode_string(&mut fields_itr)?,
            xyz_challenge: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(verify_and_auth_message).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn process_verify_completed(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let _is_successful_str = decode_string(&mut fields_itr)?;
        let is_successful = "true" == decode_string(&mut fields_itr)?;
        let error_text = decode_string(&mut fields_itr)?;
        let verify_completed = ServerRspMsg::VerifyCompleted {
            is_successful,
            error_text,
        };

        self.send_queue.send(verify_completed).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    #[allow(dead_code)]
    fn process_verify_message_api(&mut self, fields: &[String]) -> Result<(), IBKRApiLibError> {
        let mut fields_itr = fields.iter();
        //throw away message_id
        fields_itr.next();
        //throw away version
        fields_itr.next();

        let verify_message_api = ServerRspMsg::VerifyMessageApi {
            api_data: decode_string(&mut fields_itr)?,
        };

        self.send_queue.send(verify_message_api).unwrap();
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    fn read_last_trade_date(
        &self,
        contract: &mut ContractDetails,
        is_bond: bool,
        read_date: &str,
    ) -> Result<(), IBKRApiLibError> {
        if read_date != "" {
            let splitted = read_date.split_whitespace().collect::<Vec<&str>>();
            if splitted.len() > 0 {
                if is_bond {
                    contract.maturity = splitted.get(0).unwrap_or_else(|| &"").to_string();
                } else {
                    contract.contract.last_trade_date_or_contract_month =
                        splitted.get(0).unwrap_or_else(|| &"").to_string();
                }
            }
            if splitted.len() > 1 {
                contract.last_trade_time = splitted.get(1).unwrap_or_else(|| &"").to_string();
            }
            if is_bond && splitted.len() > 2 {
                contract.time_zone_id = splitted.get(2).unwrap_or_else(|| &"").to_string();
            }
        }
        Ok(())
    }

    //----------------------------------------------------------------------------------------------
    pub fn run(&mut self) -> Result<(), IBKRApiLibError> {
        // This is the function that has the message loop.
        const CONN_STATE_POISONED: &str = "Connection state mutex was poisoned";
        //let connection_closed = ServerRspMsg::ConnectionClosed;
        info!("Starting run...");
        // !self.done &&
        loop {
            // debug!("Client waiting for message...");
            let text = self.msg_queue.recv();
            match text {
                Result::Ok(val) => {
                    if val.len() > MAX_MSG_LEN as usize {
                        let error_msg = ServerRspMsg::ErrMsg {
                            req_id: NO_VALID_ID,
                            error_code: TwsError::NotConnected.code(),
                            error_str: format!(
                                "{}:{}:{}",
                                TwsError::NotConnected.message(),
                                val.len(),
                                val
                            )
                            .to_string(),
                        };

                        self.send_queue.send(error_msg).unwrap();
                        error!("Error receiving message.  Disconnected: Message too big");
                        //self.send_queue.send(connection_closed).unwrap();
                        *self.conn_state.lock().expect(CONN_STATE_POISONED) =
                            ConnStatus::DISCONNECTED;
                        error!("Error receiving message.  Invalid size.  Disconnected.");
                        return Ok(());
                    } else {
                        let fields = read_fields((&val).as_ref());
                        self.interpret(fields.as_slice())?;
                    }
                }
                Result::Err(err) => {
                    if *self.conn_state.lock().expect(CONN_STATE_POISONED).deref() as i32
                        != ConnStatus::DISCONNECTED as i32
                    {
                        info!("Error receiving message.  Disconnected: {:?}", err);
                        //self.send_queue.send(connection_closed).unwrap();
                        *self.conn_state.lock().expect(CONN_STATE_POISONED) =
                            ConnStatus::DISCONNECTED;

                        return Ok(());
                    } else {
                        error!("Disconnected...");
                        return Ok(());
                    }
                }
            }
        }
    }
}
