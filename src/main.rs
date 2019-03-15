use serde::{Deserialize, Serialize};

use concourse_resource::*;

mod message;
use message::Message;

struct Test {}

#[derive(Serialize, Deserialize, Debug)]
struct Version {
    ver: String,
}

#[derive(Deserialize, Debug)]
struct Source {
    url: String,
    channel: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum AlertType {
    Success,
    Failed,
    Started,
    Aborted,
    // Fixed,
    // Broke,
    Custom,
}

impl AlertType {
    fn name(&self) -> &'static str {
        match self {
            AlertType::Success => "Success",
            AlertType::Failed => "Failed",
            AlertType::Started => "Started",
            AlertType::Aborted => "Aborted",
            AlertType::Custom => "Custom",
        }
    }
}

impl Default for AlertType {
    fn default() -> Self {
        AlertType::Custom
    }
}

#[derive(Deserialize, Debug)]
#[serde(default)]
struct OutParams {
    alert_type: AlertType,
    color: Option<String>,
    concise: bool,
    message: Option<String>,
    channel: Option<String>,
}

impl Default for OutParams {
    fn default() -> Self {
        Self {
            alert_type: AlertType::default(),
            color: None,
            concise: false,
            message: None,
            channel: None,
        }
    }
}

#[derive(Serialize, Debug)]
struct OutMetadata {
    sent: bool,
    channel: Option<String>,
    #[serde(rename = "type")]
    alert_type: Option<AlertType>,
    error: Option<String>,
}

fn try_to_send(url: &str, message: &slack_push::Message) -> Result<(), String> {
    reqwest::Client::new()
        .post(reqwest::Url::parse(url).map_err(|err| format!("{}", err))?)
        .json(message)
        .send()
        .map_err(|err| format!("{}", err))?
        .text()
        .map_err(|err| format!("{}", err))?;
    Ok(())
}

// #[derive(Debug)]
// struct Error {}
// impl std::error::Error for Error {}
// impl std::fmt::Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "Error fetching version")
//     }
// }

impl Resource for Test {
    type Source = Source;
    type Version = Version;

    type InParams = ();
    type InMetadata = ();
    type OutParams = OutParams;
    type OutMetadata = OutMetadata;

    fn resource_check(
        _source: Self::Source,
        _version: Option<Self::Version>,
    ) -> Vec<Self::Version> {
        vec![]
    }

    fn resource_in(
        _source: Self::Source,
        _version: Self::Version,
        _params: Option<Self::InParams>,
        _output_path: &str,
    ) -> Result<InOutput<Self::Version, Self::InMetadata>, Box<std::error::Error>> {
        Ok(InOutput {
            version: Self::Version {
                ver: String::from("static"),
            },
            metadata: None,
        })
    }

    fn resource_out(
        source: Self::Source,
        params: Option<Self::OutParams>,
        input_path: &str,
    ) -> OutOutput<Self::Version, Self::OutMetadata> {
        let metadata = if let Some(params) = params {
            let params = Self::OutParams {
                channel: source.channel,
                ..params
            };
            let message =
                Message::new(&params, input_path).to_slack_message(Self::build_metadata(), &params);
            if let Result::Err(error) = try_to_send(&source.url, &message) {
                Some(OutMetadata {
                    alert_type: Some(params.alert_type),
                    channel: params.channel,
                    sent: false,
                    error: Some(error),
                })
            } else {
                Some(OutMetadata {
                    alert_type: Some(params.alert_type),
                    channel: params.channel,
                    sent: true,
                    error: None,
                })
            }
        } else {
            Some(OutMetadata {
                alert_type: None,
                channel: None,
                sent: false,
                error: Some(String::from("invalid parameters")),
            })
        };
        OutOutput {
            version: Self::Version {
                ver: String::from("static"),
            },
            metadata: metadata,
        }
    }
}

create_resource!(Test);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_deserialize_params() {
        let params = r#"{}"#;

        let params = serde_json::from_str::<OutParams>(params);
        assert!(dbg!(params).is_ok());
    }
}
