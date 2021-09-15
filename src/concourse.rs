use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    Started,
    Pending,
    Succeeded,
    Failed,
    Errored,
    Aborted,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Build {
    pub(crate) status: Option<Status>,
    // id: u32,
    // team_name: String,
    // name: String,
    // job_name: String,
    // api_url: String,
    // pipeline_name: String,
    // start_time: u64,
    // end_time: u64,
}

pub(crate) struct Concourse {
    url: String,
    bearer: String,
    ssl_configuration: Option<super::SslConfiguration>,
    client: Option<reqwest::blocking::Client>,
}

#[derive(Serialize, Debug)]
struct TokenRequest<'a> {
    grant_type: &'a str,
    username: &'a str,
    password: &'a str,
    scope: &'a str,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    id_token: Option<String>,
}

impl Concourse {
    pub(crate) fn new(url: &str) -> Self {
        Self {
            url: if url.ends_with('/') {
                String::from(url)
            } else {
                format!("{}/", url)
            },
            bearer: String::from(""),
            ssl_configuration: None,
            client: None,
        }
    }

    pub(crate) fn auth(mut self, username: &str, password: &str) -> Self {
        if let Ok(token) = reqwest::Url::parse(&format!("{}sky/issuer/token", self.url))
            .map_err(|_| ())
            .and_then(|url| {
                self.client
                    .clone()
                    .expect("error configuring HTTP client")
                    .post(url)
                    .basic_auth("fly", Some("Zmx5"))
                    .form(&TokenRequest {
                        grant_type: "password",
                        username,
                        password,
                        scope: "openid profile email groups federated:id",
                    })
                    .send()
                    .map_err(|_| ())
                    .and_then(|req| req.json::<TokenResponse>().map_err(|_| ()))
            })
        {
            self.bearer = token.access_token;
        } else {
            self = self.old_auth(username, password);
        }
        self
    }

    pub(crate) fn old_auth(mut self, username: &str, password: &str) -> Self {
        if let Ok(token) = reqwest::Url::parse(&format!("{}sky/token", self.url))
            .map_err(|_| ())
            .and_then(|url| {
                self.client
                    .clone()
                    .expect("error configuring HTTP client")
                    .post(url)
                    .basic_auth("fly", Some("Zmx5"))
                    .form(&TokenRequest {
                        grant_type: "password",
                        username,
                        password,
                        scope: "openid+profile+email+groups+federated:id",
                    })
                    .send()
                    .map_err(|_| ())
                    .and_then(|req| req.json::<TokenResponse>().map_err(|_| ()))
            })
        {
            self.bearer = token.access_token;
        }
        self
    }

    pub(crate) fn is_authed(&self) -> bool {
        !self.bearer.is_empty()
    }

    pub(crate) fn ssl_configuration(mut self, ssl_configuration: super::SslConfiguration) -> Self {
        self.ssl_configuration = Some(ssl_configuration);
        self
    }

    pub(crate) fn build(mut self) -> Self {
        let mut client = reqwest::blocking::Client::builder();
        if let Some(true) = self.ssl_configuration.as_ref().and_then(|c| c.ignore_ssl) {
            client = client.danger_accept_invalid_certs(true);
        }
        if let Some(ca_cert) = self
            .ssl_configuration
            .as_ref()
            .and_then(|c| c.ca_cert.as_ref())
        {
            client = client.add_root_certificate(
                reqwest::Certificate::from_pem(ca_cert.as_bytes())
                    .expect("error reading CA certificate"),
            );
        }
        self.client = Some(client.build().expect("error configuring HTTP client"));
        self
    }

    pub(crate) fn get_build(
        self,
        team: &str,
        pipeline: &str,
        job: &str,
        build: u32,
        debug: bool,
    ) -> Option<Build> {
        reqwest::Url::parse(&format!(
            "{}api/v1/teams/{}/pipelines/{}/jobs/{}/builds/{}",
            self.url, team, pipeline, job, build
        ))
        .map_err(|_| ())
        .and_then(|url| {
            if debug {
                eprintln!("getting url {:?}", url);
            }
            let mut req = self.client.as_ref().expect("error configuring HTTP client").get(url);
            let authed = self.is_authed();
            let token = &self.bearer;
            if authed {
                req = req.bearer_auth(token);
                if debug {
                    eprintln!("token {:?}", token);
                }
            }

            req.send()
                .map_err(|err| {
                    if debug {
                        eprintln!("got an error getting build: {:?}", err);
                    }
                })
                .and_then(|req| {
                    if debug {
                        eprintln!("response: {:?}", req.status());
                    }
                    req.json::<Build>().map_err(|err| {
                        if debug {
                            eprintln!("got an error deserializing: {:}", err)
                        }
                    })
                })
        })
        .ok()
    }
}
