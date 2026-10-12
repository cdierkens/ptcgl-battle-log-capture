use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use reqwest::redirect::Policy;

use crate::ports::{DeliveryRequest, DeliveryTransport};

const TIMEOUT: Duration = Duration::from_secs(30);

pub struct HttpTransport {
    client: Client,
}

impl HttpTransport {
    /// Build on the thread that sends: the blocking client must not be made inside an async runtime.
    pub fn new(app_version: &str) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(TIMEOUT)
            // A redirect could carry the token to another host.
            .redirect(Policy::none())
            .user_agent(format!("PTCGL-Tracker/{app_version}"))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }
}

impl DeliveryTransport for HttpTransport {
    fn post(&self, request: &DeliveryRequest<'_>) -> Result<u16, String> {
        let mut builder = self
            .client
            .post(request.url)
            .bearer_auth(request.token)
            .header(CONTENT_TYPE, "application/json")
            .body(request.body.to_owned());
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        builder.send().map(|response| response.status().as_u16()).map_err(|e| e.to_string())
    }
}
