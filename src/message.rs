use serde::Serialize;

use crate::{AlertType, OutParams};
use concourse_resource::BuildParameters;

#[derive(Serialize)]
pub struct Message {
    pub color: String,
    pub text: Option<String>,
    pub icon_url: String,
}

fn name_and_url_from_params(build_metadata: &BuildParameters) -> (Option<String>, Option<String>) {
    if let (Some(build_pipeline_name), Some(build_job_name), Some(build_name)) = (
        build_metadata.build_pipeline_name.as_ref(),
        build_metadata.build_job_name.as_ref(),
        build_metadata.build_name.as_ref(),
    ) {
        let job_name = format!("{}/{} #{}", build_pipeline_name, build_job_name, build_name,);
        let build_url = format!(
            "{}/teams/{}/pipelines/{}/jobs/{}/builds/{}",
            build_metadata.atc_external_url,
            urlencoding::encode(&build_metadata.build_team_name),
            urlencoding::encode(&build_pipeline_name),
            urlencoding::encode(&build_job_name),
            urlencoding::encode(&build_name),
        );
        (Some(job_name), Some(build_url))
    } else {
        (Some(String::from("unknown job")), None)
    }
}

impl Message {
    pub(crate) fn new(params: &OutParams) -> Message {
        let mut message = match params.alert_type {
            AlertType::Success => Message {
                color: String::from("#32cd32"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-succeeded.png",
                ),
                text: None,
            },
            AlertType::Failed => Message {
                color: String::from("#d00000"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-failed.png",
                ),
                text: None,
            },
            AlertType::Started => Message {
                color: String::from("#f7cd42"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-started.png",
                ),
                text: None,
            },
            AlertType::Aborted => Message {
                color: String::from("#8d4b32"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-aborted.png",
                ),
                text: None,
            },
            AlertType::Custom => Message {
                color: String::from("#35495c"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-pending.png",
                ),
                text: None,
            },
        };
        if let Some(color) = params.color.as_ref() {
            message.color = color.clone();
        }
        if let Some(text) = params.message.as_ref() {
            let mut path = std::path::PathBuf::new();
            path.push("/tmp/build/put/");
            path.push(text);
            message.text = Some(std::fs::read_to_string(path).unwrap_or_else(|_| text.clone()))
        }
        message
    }

    pub(crate) fn to_slack_message(
        self,
        build_metadata: BuildParameters,
        params: OutParams,
    ) -> slack_push::Message {
        let (job_name, build_url) = name_and_url_from_params(&build_metadata);
        slack_push::Message {
            attachments: Some(vec![slack_push::message::Attachment {
                author_name: if params.concise {
                    job_name
                } else {
                    self.text
                        .clone()
                        .or_else(|| Some(String::from(params.alert_type.name())))
                },
                text: if params.concise { self.text } else { None },
                color: Some(self.color),
                footer: build_url,
                footer_icon: Some(self.icon_url),
                fields: if params.concise {
                    None
                } else {
                    Some(vec![
                        slack_push::message::AttachmentField {
                            title: Some(String::from("Job")),
                            value: Some(format!(
                                "{}/{}",
                                build_metadata
                                    .build_pipeline_name
                                    .as_ref()
                                    .map(String::as_ref)
                                    .unwrap_or("unknown-pipeline"),
                                build_metadata
                                    .build_job_name
                                    .unwrap_or_else(|| String::from("unknown-job"))
                            )),
                            short: Some(true),
                        },
                        slack_push::message::AttachmentField {
                            title: Some(String::from("Build")),
                            value: Some(
                                build_metadata
                                    .build_name
                                    .unwrap_or_else(|| String::from("unknown-build")),
                            ),
                            short: Some(true),
                        },
                    ])
                },
                ..Default::default()
            }]),
            channel: params.channel,

            ..Default::default()
        }
    }
}
