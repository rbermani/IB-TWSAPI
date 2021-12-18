#[cfg(test)]
mod tests {
    use crate::core::client::{ConnStatus, EClient, POISONED_MUTEX};

    use crate::core::{
        execution::{ExecutionFilter},
        streamer::{Streamer, TestStreamer},
    };
    use crate::{
        core::{
            errors::IBKRApiLibError,
            messages::{read_fields, read_msg, ServerReqMsgDiscriminants},
        },
        examples::contract_samples::simple_future,
    };
    use std::sync::{Arc, Mutex};

    //------------------------------------------------------------------------------------------------
    trait ClientConnectForTest {
        fn connect_test(&mut self);
    }

    impl ClientConnectForTest for EClient {
        fn connect_test(&mut self) {
            *self.conn_state.lock().expect(POISONED_MUTEX) = ConnStatus::CONNECTED;
            let streamer = TestStreamer::new();
            self.set_streamer(Option::from(Box::new(streamer) as Box<dyn Streamer>));
            self.server_version = 151;
        }
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_account_summary() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 2;
        let req_id = 100;
        let group_name = "MyGroup";
        let tags = "tag1:tag_value1, tag2:tag_value2";
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_account_summary(req_id, group_name, tags)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 54] = [
            0, 0, 0, 50, 54, 50, 0, 50, 0, 49, 48, 48, 0, 77, 121, 71, 114, 111, 117, 112, 0, 116,
            97, 103, 49, 58, 116, 97, 103, 95, 118, 97, 108, 117, 101, 49, 44, 32, 116, 97, 103,
            50, 58, 116, 97, 103, 95, 118, 97, 108, 117, 101, 50, 0,
        ];

        let msg_data = read_msg(buf.as_slice())?;

        let fields = read_fields(&msg_data.1);
        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqAccountSummary as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());
        assert_eq!(req_id, fields[2].parse::<i32>().unwrap());
        assert_eq!(group_name, fields[3]);
        assert_eq!(tags, fields[4]);

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_account_updates() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 2;
        let subscribe = true;
        let acct_code = "D12345";
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_account_updates(subscribe, acct_code)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 17] = [0, 0, 0, 13, 54, 0, 50, 0, 49, 0, 68, 49, 50, 51, 52, 53, 0];

        let msg_data = read_msg(buf.as_slice())?;
        let fields = read_fields(&msg_data.1);
        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqAcctData as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());
        assert_eq!(subscribe as i32, fields[2].parse::<i32>().unwrap());
        assert_eq!(acct_code, fields[3]);

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_account_updates_multi() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 1;
        let req_id = 101;
        let acct_code = "D12345";
        let model_code = "ABC";
        let ledger_and_nvl = true;
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_account_updates_multi(req_id, acct_code, model_code, ledger_and_nvl)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 26] = [
            0, 0, 0, 22, 55, 54, 0, 49, 0, 49, 48, 49, 0, 68, 49, 50, 51, 52, 53, 0, 65, 66, 67, 0,
            49, 0,
        ];

        let msg_data = read_msg(buf.as_slice())?;

        let fields = read_fields(&msg_data.1);
        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqAccountUpdatesMulti as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());
        assert_eq!(req_id, fields[2].parse::<i32>().unwrap());
        assert_eq!(acct_code, fields[3]);
        assert_eq!(model_code, fields[4]);
        assert_eq!(ledger_and_nvl as i32, fields[5].parse::<i32>().unwrap());

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_all_open_orders() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 1;

        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_all_open_orders()?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 9] = [0, 0, 0, 5, 49, 54, 0, 49, 0];

        let msg_data = read_msg(buf.as_slice())?;
        let fields = read_fields(&msg_data.1);

        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqAllOpenOrders as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_auto_open_orders() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 1;
        let auto_bind = true;
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_auto_open_orders(auto_bind)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 11] = [0, 0, 0, 7, 49, 53, 0, 49, 0, 49, 0];

        let msg_data = read_msg(buf.as_slice())?;

        let fields = read_fields(&msg_data.1);

        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqAutoOpenOrders as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_completed_orders() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 1;
        let api_only = true;
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_completed_orders(api_only)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 9] = [0, 0, 0, 5, 57, 57, 0, 49, 0];

        let msg_data = read_msg(buf.as_slice())?;

        let fields = read_fields(&msg_data.1);

        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqCompletedOrders as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_contract_details() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 8;
        let req_id = 102;
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        let contract = simple_future();
        locked_app.req_contract_details(req_id, &contract)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 50] = [
            0, 0, 0, 46, 57, 0, 56, 0, 49, 48, 50, 0, 48, 0, 69, 83, 0, 70, 85, 84, 0, 50, 48, 50,
            48, 48, 57, 0, 48, 0, 0, 0, 71, 76, 79, 66, 69, 88, 0, 0, 85, 83, 68, 0, 0, 0, 48, 0,
            0, 0,
        ];

        let msg_data = read_msg(buf.as_slice())?;
        let fields = read_fields(&msg_data.1);

        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqContractData as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());
        assert_eq!(req_id, fields[2].parse::<i32>().unwrap());
        assert_eq!(contract.con_id, fields[3].parse::<i32>().unwrap()); // srv v37 and above
        assert_eq!(contract.symbol, fields[4]);

        assert_eq!(contract.sec_type, fields[5]);
        assert_eq!(contract.last_trade_date_or_contract_month, fields[6]);
        assert_eq!(contract.strike, fields[7].parse::<f64>().unwrap());
        assert_eq!(contract.right, fields[8]);
        assert_eq!(contract.multiplier, fields[9]); // srv v15 and above

        assert_eq!(contract.exchange, fields[10]);
        assert_eq!(contract.primary_exchange, fields[11]);

        assert_eq!(contract.currency, fields[12]);
        assert_eq!(contract.local_symbol, fields[13]);

        assert_eq!(contract.trading_class, fields[14]);
        assert_eq!(
            contract.include_expired as i32,
            fields[15].parse::<i32>().unwrap()
        ); // srv v31 and above

        assert_eq!(contract.sec_id_type, fields[16]);
        assert_eq!(contract.sec_id, fields[17]);

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_current_time() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 2;

        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();
        locked_app.req_current_time()?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 9] = [0, 0, 0, 5, 52, 57, 0, 50, 0];

        let msg_data = read_msg(buf.as_slice())?;

        let fields = read_fields(&msg_data.1);

        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqCurrentTime as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());

        Ok(())
    }

    //------------------------------------------------------------------------------------------------
    #[test]
    fn test_req_executions() -> Result<(), IBKRApiLibError> {
        let app = Arc::new(Mutex::new(EClient::new()));

        let version = 3;
        let req_id = 102;
        let client_id = 0;
        let acct_code = "D54321";

        //Time from which the executions will be returned yyyymmdd hh:mm:ss Only those executions reported after the specified time will be returned.
        let time = "";
        let symbol = "ES";
        let sec_type = "FUT";
        let exchange = "GLOBEX";
        let side = "BUY";
        let mut buf = Vec::<u8>::new();

        let mut locked_app = app.lock().expect("EClient mutex was poisoned");

        locked_app.connect_test();

        let exec_filter = ExecutionFilter::new(
            client_id,
            acct_code.to_string(),
            time.to_string(),
            symbol.to_string().to_string(),
            sec_type.to_string(),
            exchange.to_string(),
            side.to_string(),
        );
        locked_app.req_executions(req_id, &exec_filter)?;
        locked_app.stream.as_mut().unwrap().read_to_end(&mut buf)?;

        let expected: [u8; 40] = [
            0, 0, 0, 36, 55, 0, 51, 0, 49, 48, 50, 0, 48, 0, 68, 53, 52, 51, 50, 49, 0, 0, 69, 83,
            0, 70, 85, 84, 0, 71, 76, 79, 66, 69, 88, 0, 66, 85, 89, 0,
        ];

        let msg_data = read_msg(buf.as_slice())?;
        //println!("read message: {:?}", read_msg(buf.as_slice())?);
        let fields = read_fields(&msg_data.1);
        //println!("read fields: {:?}", read_fields(&msg_data.1));
        assert_eq!(expected.as_ref(), buf.as_slice());
        assert_eq!(
            ServerReqMsgDiscriminants::ReqExecutions as u8,
            fields[0].parse::<u8>().unwrap()
        );
        assert_eq!(version, fields[1].parse::<i32>().unwrap());
        assert_eq!(req_id, fields[2].parse::<i32>().unwrap());
        assert_eq!(client_id, fields[3].parse::<i32>().unwrap());
        assert_eq!(acct_code, fields[4]);
        assert_eq!(time, fields[5]);
        assert_eq!(symbol, fields[6]);
        assert_eq!(sec_type, fields[7]);
        assert_eq!(exchange, fields[8]);
        assert_eq!(side, fields[9]);

        Ok(())
    }
}
