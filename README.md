# concourse-slack-notifier [![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-yellow.svg)](https://opensource.org/licenses/Apache-2.0) [![Build Status](https://travis-ci.org/mockersf/concourse-slack-notifier.svg?branch=master)](https://travis-ci.org/mockersf/concourse-slack-notifier) [![Docker Pulls](https://img.shields.io/docker/pulls/mockersf/concourse-slack-notifier.svg)](https://hub.docker.com/r/mockersf/concourse-slack-notifier/) [![Image Informtations](https://images.microbadger.com/badges/image/mockersf/concourse-slack-notifier.svg)](https://microbadger.com/images/mockersf/concourse-slack-notifier)

A structured and opinionated Slack notification resource for [Concourse](https://concourse.ci/). It started as a rewrite of [arbourd/concourse-slack-alert-resource](https://github.com/arbourd/concourse-slack-alert-resource) in Rust to add a few features, mainly reading the message from a file or being able to send shorter messages that still had a similar format.

<img src="./img/custom.png">

The message is built by using Concourse's [resource metadata](https://concourse-ci.org/implementing-resources.html#resource-metadata) to show the pipeline, job, build number and a URL.

## Installing

Use this resource by adding the following to the resource_types section of a pipeline config:

```yaml
resource_types:

- name: slack-notifier
  type: docker-image
  source:
    repository: mockersf/concourse-slack-notifier
```

See the [Concourse docs](https://concourse-ci.org/resource-types.html) for more details on adding `resource_types` to a pipeline config.

## Source Configuration

* `url`: *Required.* Slack webhook URL.
* `channel`: *Optional*. Target channel where messages are posted. If unset the default channel of the webhook is used.
* `concourse_url`: *Optional.* The external URL that points to Concourse. Defaults to the env variable `ATC_EXTERNAL_URL`.
* `username`: *Optional.* Concourse local user (or basic auth) username. Required for non-public pipelines if using alert type `fixed` or `broke`
* `password`: *Optional.* Concourse local user (or basic auth) password. Required for non-public pipelines if using alert type `fixed` or `broke`

```yaml
resources:

- name: notify
  type: slack-notifier
  source:
    url: https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX

```
## Behavior

### `check`: No operation.

### `in`: No operation.

### `out`: Send a message to Slack.

Sends a structured message to Slack based on the alert type and mode.

#### Parameters

- `alert_type`: *Optional.* The type of alert to send to Slack. See [Alert Types](#alert-types). Defaults to `custom`.
- `channel`: *Optional.* Channel where this message is posted. Defaults to the `channel` setting in Source.
- `message`: *Optional.* The status message at the top of the alert. Defaults to name of alert type. If it's a file path from an input, will use the content of the file.
- `color`: *Optional.* The color of the notification bar as a hexadecimal. Defaults to the icon color of the alert type.
- `mode`: *Optional.* The amount of information displayed in the message. See [Modes](#modes). Defaults to `normal_with_info`

basic configuration:
```yaml
jobs:
  plan:
  - put: notify
```

with an alert type, a mode and a message
```yaml
jobs:
  plan:
  - put: notify
    params:
      message: my job failed
      alert_type: failed
      mode: concise
```


#### Alert Types

- `custom`

  <img src="./img/custom.png" width="75%">

- `success`

  <img src="./img/success.png" width="75%">

- `failed`

  <img src="./img/failed.png" width="75%">

- `started`

  <img src="./img/started.png" width="75%">

- `aborted`

  <img src="./img/aborted.png" width="75%">

- `fixed`

  Fixed is a special alert type that only alerts if the previous build did not succeed. Fixed requires `username` and `password` to be set for the resource if the pipeline is not public.

  <img src="./img/fixed.png" width="75%">

- `broke`

  Broke is a special alert type that only alerts if the previous build succeed. Broke requires `username` and `password` to be set for the resource if the pipeline is not public.

  <img src="./img/broke.png" width="75%">

#### Modes

Examples notifications with a messages with the different modes:

- `concise`

  <img src="./img/concise.png" width="75%">

- `normal`

  <img src="./img/normal.png" width="75%">

- `normal_with_info`

  <img src="./img/normal_with_info.png" width="75%">
