use serde::{Deserialize, Serialize};

use concourse_resource::*;

mod message;
use message::Message;
mod concourse;

struct Test {}

#[derive(Serialize, Deserialize, Debug)]
struct Version {
    ver: String,
}

#[derive(Deserialize, Debug)]
struct Source {
    url: String,
    channel: Option<String>,
    concourse_url: Option<String>,
    #[serde(flatten)]
    credentials: Option<ConcourseCredentials>,
}

#[derive(Deserialize, Debug)]
struct ConcourseCredentials {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum AlertType {
    Success,
    Failed,
    Started,
    Aborted,
    Fixed,
    Broke,
    Custom,
}

impl AlertType {
    fn name(&self) -> &'static str {
        match self {
            AlertType::Success => "Success",
            AlertType::Failed => "Failed",
            AlertType::Started => "Started",
            AlertType::Aborted => "Aborted",
            AlertType::Fixed => "Fixed",
            AlertType::Broke => "Broke",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alert_type: Option<AlertType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Into<Vec<concourse_resource::KV>> for OutMetadata {
    fn into(self) -> Vec<concourse_resource::KV> {
        let mut md = Vec::new();

        md.push(concourse_resource::KV {
            name: String::from("sent"),
            value: if self.sent {
                String::from("true")
            } else {
                String::from("false")
            },
        });

        if let Some(channel) = self.channel {
            md.push(concourse_resource::KV {
                name: String::from("channel"),
                value: channel,
            })
        }

        if let Some(alert_type) = self.alert_type {
            md.push(concourse_resource::KV {
                name: String::from("alert_type"),
                value: String::from(alert_type.name()),
            })
        }

        if let Some(error) = self.error {
            md.push(concourse_resource::KV {
                name: String::from("error"),
                value: error,
            })
        }

        md
    }
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

impl Resource for Test {
    type Source = Source;
    type Version = Version;

    type InParams = concourse_resource::Empty;
    type InMetadata = concourse_resource::Empty;
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
        let metadata = if let Some(mut params) = params {
            if params.channel.is_none() && source.channel.is_some() {
                params.channel = source.channel.clone();
            }

            if !Self::should_send_message(&source, &params) {
                OutMetadata {
                    alert_type: Some(params.alert_type),
                    channel: params.channel,
                    sent: false,
                    error: None,
                }
            } else {
                let message = Message::new(&params, input_path)
                    .into_slack_message(Self::build_metadata(), &params);

                if let Result::Err(error) = try_to_send(&source.url, &message) {
                    OutMetadata {
                        alert_type: Some(params.alert_type),
                        channel: params.channel,
                        sent: false,
                        error: Some(error),
                    }
                } else {
                    OutMetadata {
                        alert_type: Some(params.alert_type),
                        channel: params.channel,
                        sent: true,
                        error: None,
                    }
                }
            }
        } else {
            OutMetadata {
                alert_type: None,
                channel: None,
                sent: false,
                error: Some(String::from("invalid parameters")),
            }
        };

        OutOutput {
            version: Self::Version {
                ver: String::from("static"),
            },
            metadata: Some(metadata),
        }
    }
}

impl Test {
    fn should_send_message(
        source: &<Self as Resource>::Source,
        params: &<Self as Resource>::OutParams,
    ) -> bool {
        if params.alert_type == AlertType::Broke || params.alert_type == AlertType::Fixed {
            let metadata = Self::build_metadata();
            let mut concourse = concourse::Concourse::new(
                source
                    .concourse_url
                    .as_ref()
                    .map(String::as_ref)
                    .unwrap_or(&metadata.atc_external_url),
            );

            if let Some(credentials) = &source.credentials {
                concourse = concourse.auth(&credentials.username, &credentials.password);
            }

            match (
                &params.alert_type,
                concourse
                    .get_build(
                        &metadata.team_name,
                        metadata
                            .pipeline_name
                            .as_ref()
                            .map(String::as_ref)
                            .unwrap_or(""),
                        metadata.job_name.as_ref().map(String::as_ref).unwrap_or(""),
                        metadata.name.unwrap_or(1) - 1,
                    )
                    .and_then(|b| b.status),
            ) {
                (AlertType::Broke, Some(concourse::Status::Succeeded)) => true,
                (AlertType::Fixed, Some(concourse::Status::Succeeded)) => false,
                (AlertType::Fixed, Some(_)) => true,
                (_, _) => false,
            }
        } else {
            true
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
