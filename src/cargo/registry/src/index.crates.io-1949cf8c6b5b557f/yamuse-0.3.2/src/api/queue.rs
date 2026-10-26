//! playback-queue endpoints.
//!
//! queues are device-scoped: only the device that created a queue may update it,
//! which is why every call here carries the device descriptor in a header.

use serde_json::Value;

use crate::client::Client;
use crate::error::Result;
use crate::models::{Mutation, Queue, QueueItem};
use crate::params::{Params, segment};

/// header carrying the device descriptor for queue requests.
const DEVICE_HEADER: &str = "X-Yandex-Music-Device";

impl Client {
    /// fetch the list of the user's queues.
    ///
    /// `GET /queues`
    pub async fn queues(&self, device: Option<&str>) -> Result<Vec<QueueItem>> {
        let device = device.unwrap_or_else(|| self.device());
        let value: Value = self
            .get_with_headers("/queues", Params::new(), &[(DEVICE_HEADER, device)])
            .await?;
        // the payload nests the list under `queues` on some responses and returns a
        // bare array on others.
        let queues = value.get("queues").cloned().unwrap_or(value);
        self.decode(queues)
    }

    /// fetch a single queue by id.
    ///
    /// `GET /queues/{id}`
    pub async fn queue(&self, queue_id: &str) -> Result<Queue> {
        let queue_id = segment(queue_id);
        self.get(&format!("/queues/{queue_id}"), Params::new())
            .await
    }

    /// update the current playing position of a queue.
    ///
    /// `POST /queues/{id}/update-position`
    pub async fn update_queue_position(
        &self,
        queue_id: &str,
        current_index: i64,
        device: Option<&str>,
    ) -> Result<Mutation> {
        let device = device.unwrap_or_else(|| self.device());
        let queue_id = segment(queue_id);
        let path = format!("/queues/{queue_id}/update-position");
        let query = Params::new().add("currentIndex", current_index);
        let form = Params::new().add("isInteractive", false);
        let value = self
            .post_form_value_with(&path, &query, form, &[(DEVICE_HEADER, device)])
            .await?;
        Client::acknowledged(&value)
    }

    /// create a new playback queue.
    ///
    /// `POST /queues`
    ///
    /// the body is json, not a form: tagging it as form-urlencoded makes this
    /// endpoint drop the connection.
    ///
    /// returns the id of the created queue.
    pub async fn create_queue(
        &self,
        queue: &Value,
        device: Option<&str>,
    ) -> Result<Option<String>> {
        let device = device.unwrap_or_else(|| self.device());
        let body = serde_json::to_string(queue)?;
        let value = self
            .post_raw_json_value("/queues", body, &[(DEVICE_HEADER, device)])
            .await?;
        Ok(value
            .get("id")
            .and_then(Value::as_str)
            .map(std::borrow::ToOwned::to_owned))
    }
}
