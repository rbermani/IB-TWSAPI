# IB-TWSAPI

This project began as a fork of the IBKR-API-Rust package, written by Brett Miller

## Notes
- Not all TWS messages have been fully tested for correctness, so this package should be considered alpha quality at best
- Send any specific, glaring bugs to me or submit a PR for review

## Changes from original package
- This implementation doesn't need generics for creating and using custom wrappers
- Only a single object needs to be instantiated instead of two
- Removed ArcMutex idiom around EClient and Wrapper objects
- Uses a channel-based event dispatcher instead of a call based one
- Migrated to using the rust_decimal package instead of BigDecimal

## Instructions
- Copy the test_wrapper.rs to your project, rename, and re-implement functionality as needed.
- Write application using your own implementation.

## Original package description
Port of Interactive Broker's trading API written in Rust (API_Version=9.76.01)

Please see the latest IB Tws Api documentation here: <http://interactivebrokers.github.io/tws-api/introduction.html>.

The documentation has information regarding configuring Trader WorkStation and IB Gateway to enable API access.

For usage of this library, please see the example implementation in [src/examples/test_helpers/manual_tests.rs](src/bin/manual_tests.rs)

~~The main structs and traits that clients will use are [**EClient**](src/core/client.rs) , a struct that is responsible for connecting to TWS or IB Gateway and sending requests,  and [**Wrapper**](src/core/wrapper.rs), a trait that clients will implement that declares callback functions that get called when the application receives messages from TWS/IB Gateway.~~

## Example

In the example below, TWS will send the next valid order ID when the sample application connects. This will cause the ***Wrapper*** callback method
***next_valid_id*** to be called, which will start sending test requests to TWS (see the
***start_requests*** method in ***TestWrapper*** which is called by ***next_valid_id***).

```rust, no_run
use log::*;
use std::thread;
use std::time::Duration;
use ibtwsapi::core::errors::*;
use ibtwsapi::examples::example_wrapper::ExampleWrapper;

pub fn main() -> Result<(), IBKRApiLibError> {
    match log4rs::init_file("./log_config.yml", Default::default()) {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}", e.to_string());
            return Err(IBKRApiLibError::ApiError(TwsApiReportableError::new(
                -1,
                "-1".to_string(),
                "Failed to create logger!!".to_string(),
            )))
        }
    };

    let mut app = ExampleWrapper::new();

    info!("getting connection...");

    //use port 7497 for TWS or 4002 for IB Gateway, depending on the port you have set
    app.client.connect("127.0.0.1", 4002, 0)?;
    loop {
        match app.process_event() {
            Ok(_) => continue,
            Err(e) =>
            {
                error!("{}", e.to_string());
                break ();
            },
        };
    }
    thread::sleep(Duration::new(2, 0));

    Ok(())
}
```

## TODO
- [ ] Perform profiling and collect performance data
- [ ] Run rust-clippy linter


## DISCLAIMER

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
