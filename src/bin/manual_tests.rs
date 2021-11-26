//! Binary for manually testing crate

use ibtwsapi::core::errors::*;
use ibtwsapi::examples::example_wrapper::ExampleWrapper;
use log::*;
use std::thread;
use std::time::Duration;

/// Example of using client and wrapper.
/// Requires a running instance of TWS or IB Gateway connected to the port in main.
/// Upon connecting, TWS will send the next valid order ID which will cause the wrapper callback method
/// next_valid_id to be called, which will start sending tests requests to TWS (see the
/// start_requests function in ExampleWrapper which is called by next_valid_id
//==================================================================================================
pub fn main() -> Result<(), IBKRApiLibError> {
    match log4rs::init_file("./log_config.yml", Default::default()) {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}", e.to_string());
            return Err(IBKRApiLibError::ApiError(TwsApiReportableError::new(
                -1,
                "-1".to_string(),
                "Failed to create logger!!".to_string(),
            )));
        }
    };

    let mut app = ExampleWrapper::new();

    info!("getting connection...");

    //use port 7497 for TWS or 4002 for IB Gateway, depending on the port you have set
    app.client.connect("127.0.0.1", 4002, 0)?;
    loop {
        match app.process_event() {
            Ok(_) => continue,
            Err(e) => {
                error!("{}", e.to_string());
                break ();
            }
        };
    }
    thread::sleep(Duration::new(2, 0));

    Ok(())
}
