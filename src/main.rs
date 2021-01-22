use serde::{Deserialize, Serialize};

use concourse_resource::*;

mod message;
use message::Message;
mod concourse;

struct SlackNotifier {}

#[derive(Serialize, Deserialize, Debug)]
struct Version {
    status: String,
}

#[derive(Deserialize, Debug)]
struct Source {
    url: String,
    channel: Option<String>,
    concourse_url: Option<String>,
    #[serde(flatten)]
    credentials: Option<ConcourseCredentials>,
    #[serde(flatten)]
    ssl_configuration: Option<SslConfiguration>,
    disabled: Option<bool>,
    debug: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
struct SslConfiguration {
    ca_cert: Option<String>,
    ignore_ssl: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
struct ClientCert {
    cert: String,
    key: String,
}

#[derive(Deserialize, Debug)]
struct ConcourseCredentials {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum AlertType {
    Success,
    Failed,
    Started,
    Aborted,
    Errored,
    Fixed,
    Broke,
    Custom,
}

impl AlertType {
    fn message(&self) -> &'static str {
        match self {
            AlertType::Success => "Success",
            AlertType::Failed => "Failed",
            AlertType::Started => "Started",
            AlertType::Aborted => "Aborted",
            AlertType::Errored => "Errored",
            AlertType::Fixed => "Fixed",
            AlertType::Broke => "Broke",
            AlertType::Custom => "Build Finished",
        }
    }
}

impl Default for AlertType {
    fn default() -> Self {
        AlertType::Custom
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Concise,
    Normal,
    NormalWithInfo,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::NormalWithInfo
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct OutParams {
    alert_type: AlertType,
    color: Option<String>,
    mode: Mode,
    message: Option<String>,
    channel: Option<String>,
    message_file: Option<String>,
    #[serde(default)]
    fail_if_message_file_missing: bool,
    disabled: bool,
    message_as_code: bool,
}

#[derive(Serialize, Debug, IntoMetadataKV)]
struct OutMetadata {
    sent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alert_type: Option<AlertType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl std::fmt::Display for OutMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (
            self.sent,
            self.channel.as_ref(),
            self.alert_type.as_ref(),
            self.error.is_some(),
        ) {
            (_, _, _, true) => write!(f, "error sending notification"),
            (false, None, None, false) => write!(f, "Did not send notification"),
            (false, Some(channel), None, false) => {
                write!(f, "Did not send notification to {}", channel)
            }
            (false, None, Some(at), false) => {
                write!(f, "Did not send {} notification", at.message())
            }
            (false, Some(channel), Some(at), false) => write!(
                f,
                "Did not send {} notification to {}",
                at.message(),
                channel
            ),
            (true, None, None, false) => write!(f, "Sent notification"),
            (true, Some(channel), None, false) => write!(f, "Sent notification to {}", channel),
            (true, None, Some(at), false) => write!(f, "Sent {} notification", at.message()),
            (true, Some(channel), Some(at), false) => {
                write!(f, "Sent {} notification to {}", at.message(), channel)
            }
        }
    }
}

fn try_to_send(url: &str, message: &slack_push::Message) -> Result<(), String> {
    reqwest::blocking::Client::new()
        .post(reqwest::Url::parse(url).map_err(|err| format!("{}", err))?)
        .json(message)
        .send()
        .map_err(|err| format!("{}", err))?
        .text()
        .map_err(|err| format!("{}", err))?;
    Ok(())
}

impl Resource for SlackNotifier {
    type Source = Source;
    type Version = Version;

    type InParams = concourse_resource::Empty;
    type InMetadata = concourse_resource::Empty;
    type OutParams = OutParams;
    type OutMetadata = OutMetadata;

    fn resource_check(
        _source: Option<Self::Source>,
        _version: Option<Self::Version>,
    ) -> Vec<Self::Version> {
        vec![]
    }

    fn resource_in(
        _source: Option<Self::Source>,
        _version: Self::Version,
        _params: Option<Self::InParams>,
        _output_path: &str,
    ) -> Result<InOutput<Self::Version, Self::InMetadata>, Box<dyn std::error::Error>> {
        Ok(InOutput {
            version: Self::Version {
                status: String::from("static"),
            },
            metadata: None,
        })
    }

    fn resource_out(
        source: Option<Self::Source>,
        params: Option<Self::OutParams>,
        input_path: &str,
    ) -> OutOutput<Self::Version, Self::OutMetadata> {
        let metadata = if let Some(source) = source {
            let mut params = params.unwrap_or_default();

            if params.channel.is_none() && source.channel.is_some() {
                params.channel = source.channel.clone();
            }

            if source.debug.unwrap_or(false) {
                eprintln!("sending a message to {:?}", params.channel);
            }

            if !Self::should_send_message(&source, &params) {
                if source.debug.unwrap_or(false) {
                    eprintln!("not sending message");
                }

                OutMetadata {
                    alert_type: Some(params.alert_type),
                    channel: params.channel,
                    sent: false,
                    error: None,
                }
            } else {
                let message = Message::new(&params, input_path)
                    .into_slack_message(Self::build_metadata(), &params);

                if source.debug.unwrap_or(false) {
                    eprintln!("trying to send message {:?}", message);
                }

                if let Result::Err(error) = try_to_send(&source.url, &message) {
                    if source.debug.unwrap_or(false) {
                        eprintln!("error sending message: {:?}", error);
                    }

                    OutMetadata {
                        alert_type: Some(params.alert_type),
                        channel: params.channel,
                        sent: false,
                        error: Some(error),
                    }
                } else {
                    if source.debug.unwrap_or(false) {
                        eprintln!("message sent");
                    }
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
                error: Some(String::from("missing resource configuration")),
            }
        };

        OutOutput {
            version: Self::Version {
                status: format!("{}", metadata),
            },
            metadata: Some(metadata),
        }
    }
}

fn previous_build_name(build_name: &String) -> String {
    format!(
        "{}",
        build_name
            .parse::<f32>()
            .map(|bn| bn.trunc() as u32)
            .unwrap_or(1)
            - 1
    )
}

impl SlackNotifier {
    fn should_send_message(
        source: &<Self as Resource>::Source,
        params: &<Self as Resource>::OutParams,
    ) -> bool {
        if source.disabled.unwrap_or(false) || params.disabled {
            if source.debug.unwrap_or(false) {
                eprintln!("resource is disabled");
            }
            return false;
        }
        if params.alert_type == AlertType::Broke || params.alert_type == AlertType::Fixed {
            if source.debug.unwrap_or(false) {
                eprintln!("checking status of last build");
            }
            let metadata = Self::build_metadata();
            let mut concourse = concourse::Concourse::new(
                source
                    .concourse_url
                    .as_ref()
                    .map(String::as_ref)
                    .unwrap_or(&metadata.atc_external_url),
            );

            if let Some(ssl_configuration) = source.ssl_configuration.as_ref() {
                concourse = concourse.ssl_configuration(ssl_configuration.clone());
            }

            concourse = concourse.build();

            if let Some(credentials) = &source.credentials {
                concourse = concourse.auth(&credentials.username, &credentials.password);
                if source.debug.unwrap_or(false) {
                    eprintln!("authenticated to concourse: {}", concourse.is_authed());
                }
            }

            if source.debug.unwrap_or(false) {
                eprintln!(
                    "getting build {:?}/{:?}/{:?} #{:?}",
                    &metadata.team_name,
                    metadata
                        .pipeline_name
                        .as_ref()
                        .map(String::as_ref)
                        .unwrap_or(""),
                    metadata.job_name.as_ref().map(String::as_ref).unwrap_or(""),
                    metadata.name.as_ref().map(previous_build_name)
                );
            }

            let last_build = concourse.get_build(
                &metadata.team_name,
                metadata
                    .pipeline_name
                    .as_ref()
                    .map(String::as_ref)
                    .unwrap_or(""),
                metadata.pipeline_instance_vars,
                metadata.job_name.as_ref().map(String::as_ref).unwrap_or(""),
                metadata
                    .name
                    .as_ref()
                    .map(previous_build_name)
                    .and_then(|bn| bn.parse::<u32>().ok())
                    .unwrap_or(1),
                source.debug.unwrap_or(false),
            );

            if source.debug.unwrap_or(false) {
                eprintln!("last build: {:?}", last_build);
            }

            match (&params.alert_type, last_build.and_then(|b| b.status)) {
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

create_resource!(SlackNotifier);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_deserialize_params() {
        let params = r#"{}"#;

        let params = serde_json::from_str::<OutParams>(params);
        assert!(dbg!(params).is_ok());
    }

    #[test]
    fn can_get_previous_build_number() {
        assert_eq!(previous_build_name(&String::from("5")), "4");
        assert_eq!(previous_build_name(&String::from("6.1")), "5");
    }
}
