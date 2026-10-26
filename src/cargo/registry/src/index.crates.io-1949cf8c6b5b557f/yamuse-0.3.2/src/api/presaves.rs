//! album pre-save endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Mutation, Presaves};
use crate::params::Params;

impl Client {
    /// fetch the user's pre-saved albums.
    ///
    /// `GET /users/{uid}/presaves`
    pub async fn presaves(
        &self,
        include_released: bool,
        include_upcoming: bool,
        user_id: Option<i64>,
    ) -> Result<Presaves> {
        let uid = self.resolve_uid(user_id)?;
        let params = Params::new()
            .add("includeReleased", include_released)
            .add("includeUpcoming", include_upcoming);
        self.get(&format!("/users/{uid}/presaves"), params).await
    }

    /// pre-save an album.
    ///
    /// `POST /users/{uid}/presaves/add`
    ///
    /// with `like_after_release` the album is liked automatically once it lands.
    pub async fn add_presave(&self, album_id: i64, like_after_release: bool) -> Result<Mutation> {
        let uid = self.resolve_uid(None)?;
        let form = Params::new()
            .add("albumId", album_id)
            .add("likeAfterRelease", like_after_release);
        let value = self
            .post_form_value(&format!("/users/{uid}/presaves/add"), form)
            .await?;
        Client::acknowledged(&value)
    }

    /// remove an album pre-save.
    ///
    /// `POST /users/{uid}/presaves/remove`
    pub async fn remove_presave(&self, album_id: i64) -> Result<Mutation> {
        let uid = self.resolve_uid(None)?;
        let form = Params::new().add("albumId", album_id);
        let value = self
            .post_form_value(&format!("/users/{uid}/presaves/remove"), form)
            .await?;
        Client::acknowledged(&value)
    }
}
