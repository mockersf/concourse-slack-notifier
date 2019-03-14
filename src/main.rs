use serde::{Deserialize, Serialize};

use concourse_resource::*;

struct Test {}

#[derive(Serialize, Deserialize)]
struct Version {
    refid: String,
}

#[derive(Deserialize)]
struct Source {
    url: String,
}

#[derive(Deserialize)]
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

impl Default for AlertType {
    fn default() -> Self {
        AlertType::Custom
    }
}

#[derive(Deserialize)]
struct OutParams {
    alert_type: AlertType,
    color: Option<String>,
    concise: Option<bool>,
    message: Option<String>,
    channel: String,
}

#[derive(Serialize)]
struct Message {
    color: String,
    message: String,
    icon_url: String,
}

impl Message {
    fn new(params: &OutParams) -> Message {
        let mut message = match params.alert_type {
            AlertType::Success => Message {
                color: String::from("#32cd32"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-succeeded.png",
                ),
                message: String::from("Success"),
            },
            AlertType::Failed => Message {
                color: String::from("#d00000"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-failed.png",
                ),
                message: String::from("Failed"),
            },
            AlertType::Started => Message {
                color: String::from("#f7cd42"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-started.png",
                ),
                message: String::from("Started"),
            },
            AlertType::Aborted => Message {
                color: String::from("#8d4b32"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-aborted.png",
                ),
                message: String::from("Aborted"),
            },
            AlertType::Fixed => Message {
                color: String::from("#32cd32"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-succeeded.png",
                ),
                message: String::from("Fixed"),
            },
            AlertType::Broke => Message {
                color: String::from("#d00000"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-failed.png",
                ),
                message: String::from("Broke"),
            },
            AlertType::Custom => Message {
                color: String::from("#35495c"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-pending.png",
                ),
                message: String::from(""),
            },
        };
        if let Some(color) = params.color.as_ref() {
            message.color = color.clone();
        }
        message
    }

    fn to_message(self, build_metadata: BuildParameters, channel: String) -> slack_push::Message {
        let (text, build_url) =
            if let (Some(build_pipeline_name), Some(build_job_name), Some(build_name)) = (
                build_metadata.build_pipeline_name,
                build_metadata.build_job_name,
                build_metadata.build_name,
            ) {
                let text = format!("{}/{} #{}", build_pipeline_name, build_job_name, build_name,);
                let build_url = format!(
                    "{}/teams/{}/pipelines/{}/jobs/{}/builds/{}",
                    build_metadata.atc_external_url,
                    urlencoding::encode(&build_metadata.build_team_name),
                    urlencoding::encode(&build_pipeline_name),
                    urlencoding::encode(&build_job_name),
                    urlencoding::encode(&build_name),
                );
                (Some(text), Some(build_url))
            } else {
                (Some(String::from("unknown job")), None)
            };
        slack_push::Message {
            channel: channel,
            attachments: Some(vec![slack_push::message::Attachment {
                author_name: text,
                color: Some(self.color),
                footer: build_url,
                footer_icon: Some(self.icon_url),
                ..Default::default()
            }]),
            ..Default::default()
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
                .json(&message.to_message(Self::build_metadata(), params.channel))
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
