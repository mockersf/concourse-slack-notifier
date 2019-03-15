use serde::{Deserialize, Serialize};

use concourse_resource::*;

mod message;
use message::Message;

struct Test {}

#[derive(Serialize, Deserialize, Debug)]
struct Version {
    refid: String,
}

#[derive(Deserialize, Debug)]
struct Source {
    url: String,
}

#[derive(Deserialize, Debug)]
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

impl Resource for Test {
    type Source = Source;
    type Version = Version;

    type InParams = ();
    type InMetadata = ();
    type OutParams = OutParams;
    type OutMetadata = ();

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
        _path: &str,
    ) -> InOutput<Self::Version, Self::InMetadata> {
        InOutput {
            version: Self::Version {
                refid: String::from("static"),
            },
            metadata: None,
        }
    }

    fn resource_out(
        source: Self::Source,
        params: Option<Self::OutParams>,
    ) -> OutOutput<Self::Version, Self::OutMetadata> {
        if let Some(params) = params {
            let message = Message::new(&params);
            reqwest::Client::new()
                .post(reqwest::Url::parse(&source.url).expect("invalid WebHook URL"))
                .json(&message.to_slack_message(Self::build_metadata(), params))
                .send()
                .unwrap()
                .text()
                .unwrap();
        } else {
            eprintln!("invalid parameters");
        }
        OutOutput {
            version: Self::Version {
                refid: String::from("static"),
            },
            metadata: None,
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
