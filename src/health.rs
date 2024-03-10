use reqwest::blocking as request;

use crate::logger::alert;
use crate::system::exit;
use crate::system::Envs;

pub fn check(env: &Envs) {
    // Check if the API can be reached.
    match request::get(&format!(
        "{}/api/v3/health?apikey={}",
        &env.baseurl, &env.apikey
    )) {
        Ok(res) => {
            if res.status() != 200 {
                alert(
                    "FATAL",
                    format!("The provided \"APIKEY\" is not valid."),
                    format!(
                        "Obtain the {} API key in Settings > General > API Key",
                        &env.platform
                    ),
                    true,
                );
                exit(1);
            }
        }
        Err(error) => {
            alert(
                "FATAL",
                format!(
                    "A connection to the {} API could not be established.",
                    &env.platform
                ),
                error.to_string(),
                true,
            );
            exit(1);
        }
    }
}
