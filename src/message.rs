use serde::Serialize;

use crate::{AlertType, OutParams};
use concourse_resource::BuildMetadata;

#[derive(Serialize)]
pub struct Message {
    pub color: String,
    pub text: Option<String>,
    pub icon_url: String,
}

struct FormattedBuildInfo {
    job_name: String,
    build_name: String,
    build_number: String,
    build_url: Option<String>,
}

fn formatted_build_info_from_params(build_metadata: &BuildMetadata) -> FormattedBuildInfo {
    if let (Some(pipeline_name), Some(job_name), Some(name)) = (
        build_metadata.pipeline_name.as_ref(),
        build_metadata.job_name.as_ref(),
        build_metadata.name.as_ref(),
    ) {
        FormattedBuildInfo {
            job_name: format!("{}/{}", pipeline_name, job_name),
            build_name: format!("{}/{} #{}", pipeline_name, job_name, name,),
            build_number: format!("#{}", name),
            build_url: Some(format!(
                "{}/teams/{}/pipelines/{}/jobs/{}/builds/{}",
                build_metadata.atc_external_url,
                urlencoding::encode(&build_metadata.team_name),
                urlencoding::encode(&pipeline_name),
                urlencoding::encode(&job_name),
                name,
            )),
        }
    } else {
        FormattedBuildInfo {
            job_name: String::from("unknown job"),
            build_name: String::from("unknown build"),
            build_number: String::from("unknown build"),
            build_url: None,
        }
    }
}

impl Message {
    pub(crate) fn new(params: &OutParams, input_path: &str) -> Message {
        let mut message = match params.alert_type {
            AlertType::Success | AlertType::Fixed => Message {
                color: String::from("#11c560"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-succeeded.png",
                ),
                text: None,
            },
            AlertType::Failed | AlertType::Broke => Message {
                color: String::from("#ed4b35"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-failed.png",
                ),
                text: None,
            },
            AlertType::Started => Message {
                color: String::from("#fad43b"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-started.png",
                ),
                text: None,
            },
            AlertType::Aborted => Message {
                color: String::from("#8b572a"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-aborted.png",
                ),
                text: None,
            },
            AlertType::Errored => Message {
                color: String::from("#f5a623"),
                icon_url: String::from(
                    "https://ci.concourse-ci.org/public/images/favicon-errored.png",
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
        match (
            params.message_file.as_ref(),
            params.message.as_ref(),
            params.fail_if_message_file_missing,
        ) {
            (Some(file), Some(text), _) => {
                let mut path = std::path::PathBuf::new();
                path.push(input_path);
                path.push(file);
                message.text = Some(std::fs::read_to_string(path).unwrap_or_else(|_| text.clone()));
            }
            (Some(file), None, true) => {
                let mut path = std::path::PathBuf::new();
                path.push(input_path);
                path.push(file);
                message.text = Some(std::fs::read_to_string(path).expect("error reading file"));
            }
            (Some(file), None, false) => {
                let mut path = std::path::PathBuf::new();
                path.push(input_path);
                path.push(file);
                message.text = Some(
                    std::fs::read_to_string(path)
                        .unwrap_or_else(|_| format!("error reading file {}", file)),
                );
            }
            (None, Some(text), _) => {
                message.text = Some(text.clone());
            }
            (None, None, _) => {}
        }
        if params.message_as_code {
            message.text = message.text.map(|text| format!("```{}```", text));
        }
        message
    }

    pub(crate) fn into_slack_message(
        self,
        build_metadata: BuildMetadata,
        params: &OutParams,
    ) -> slack_push::Message {
        let formatted_build_info = formatted_build_info_from_params(&build_metadata);
        slack_push::Message {
            attachments: Some(vec![slack_push::message::Attachment {
                author_name: match params.mode {
                    crate::Mode::Concise => {
                        Some(self.text.clone().unwrap_or(formatted_build_info.build_name))
                    }
                    crate::Mode::Normal | crate::Mode::NormalWithInfo => Some(format!(
                        "{} - {}",
                        formatted_build_info.build_name,
                        params.alert_type.message()
                    )),
                },
                text: match params.mode {
                    crate::Mode::Concise => None,
                    crate::Mode::Normal | crate::Mode::NormalWithInfo => self.text,
                },
                mrkdwn_in: Some(vec![String::from("text")]),
                color: Some(self.color),
                footer: formatted_build_info.build_url,
                footer_icon: Some(self.icon_url),
                fields: match params.mode {
                    crate::Mode::Concise | crate::Mode::Normal => None,
                    crate::Mode::NormalWithInfo => Some(vec![
                        slack_push::message::AttachmentField {
                            title: Some(String::from("Job")),
                            value: Some(formatted_build_info.job_name),
                            short: Some(true),
                        },
                        slack_push::message::AttachmentField {
                            title: Some(String::from("Build")),
                            value: Some(formatted_build_info.build_number),
                            short: Some(true),
                        },
                    ]),
                },
                ..Default::default()
            }]),
            channel: params.channel.clone(),

            ..Default::default()
        }
    }
}
