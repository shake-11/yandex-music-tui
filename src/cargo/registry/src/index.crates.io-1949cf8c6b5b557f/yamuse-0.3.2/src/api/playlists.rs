//! playlist endpoints, read and write.

use crate::client::Client;
use crate::difference::{DiffTrack, Difference};
use crate::error::Result;
use crate::models::{
    GeneratedPlaylist, Mutation, Playlist, PlaylistRecommendations, PlaylistSimilarEntities,
    PlaylistTrailer, PlaylistsList, UserSettings,
};
use crate::params::{Params, segment};
use crate::types::{BatchObject, Visibility};

impl Client {
    /// fetch a user's library settings.
    ///
    /// `GET /users/{uid}/settings`
    pub async fn user_settings(&self, user_id: Option<i64>) -> Result<UserSettings> {
        let uid = self.resolve_uid(user_id)?;
        let value = self
            .get_value(&format!("/users/{uid}/settings"), Params::new())
            .await?;
        let settings = value.get("userSettings").cloned().unwrap_or(value);
        self.decode(settings)
    }

    /// fetch one of a user's playlists by kind.
    ///
    /// `GET /users/{uid}/playlists/{kind}`
    pub async fn user_playlist(
        &self,
        kind: impl std::fmt::Display,
        user_id: Option<i64>,
    ) -> Result<Playlist> {
        let uid = self.resolve_uid(user_id)?;
        let kind = segment(kind);
        self.get(&format!("/users/{uid}/playlists/{kind}"), Params::new())
            .await
    }

    /// fetch several of a user's playlists by kind.
    ///
    /// `POST /users/{uid}/playlists`
    pub async fn user_playlists(
        &self,
        kinds: impl IntoIterator<Item = impl std::fmt::Display>,
        user_id: Option<i64>,
    ) -> Result<Vec<Playlist>> {
        let uid = self.resolve_uid(user_id)?;
        let form = Params::new().add_all("kinds", kinds);
        self.post_form(&format!("/users/{uid}/playlists"), form)
            .await
    }

    /// fetch every playlist owned by a user.
    ///
    /// `GET /users/{uid}/playlists/list`
    pub async fn user_playlists_list(&self, user_id: Option<i64>) -> Result<Vec<Playlist>> {
        let uid = self.resolve_uid(user_id)?;
        self.get(&format!("/users/{uid}/playlists/list"), Params::new())
            .await
    }

    /// fetch the kinds of every playlist owned by a user.
    ///
    /// `GET /users/{uid}/playlists/list/kinds`
    pub async fn user_playlist_kinds(&self, user_id: Option<i64>) -> Result<Vec<i64>> {
        let uid = self.resolve_uid(user_id)?;
        let value = self
            .get_value(&format!("/users/{uid}/playlists/list/kinds"), Params::new())
            .await?;
        Ok(value
            .as_array()
            .map(|items| items.iter().filter_map(serde_json::Value::as_i64).collect())
            .unwrap_or_default())
    }

    /// fetch track recommendations for a playlist.
    ///
    /// `GET /users/{uid}/playlists/{kind}/recommendations`
    pub async fn playlist_recommendations(
        &self,
        kind: impl std::fmt::Display,
        user_id: Option<i64>,
    ) -> Result<PlaylistRecommendations> {
        let uid = self.resolve_uid(user_id)?;
        let kind = segment(kind);
        self.get(
            &format!("/users/{uid}/playlists/{kind}/recommendations"),
            Params::new(),
        )
        .await
    }

    /// fetch the trailer of a playlist.
    ///
    /// `GET /users/{uid}/playlists/{kind}/trailer`
    pub async fn playlist_trailer(
        &self,
        kind: impl std::fmt::Display,
        user_id: Option<i64>,
    ) -> Result<PlaylistTrailer> {
        let uid = self.resolve_uid(user_id)?;
        let kind = segment(kind);
        self.get(
            &format!("/users/{uid}/playlists/{kind}/trailer"),
            Params::new(),
        )
        .await
    }

    /// create a new playlist.
    ///
    /// `POST /users/{uid}/playlists/create`
    pub async fn create_playlist(&self, title: &str, visibility: Visibility) -> Result<Playlist> {
        let uid = self.resolve_uid(None)?;
        let form = Params::new()
            .add("title", title)
            .add("visibility", visibility);
        self.post_form(&format!("/users/{uid}/playlists/create"), form)
            .await
    }

    /// delete a playlist.
    ///
    /// `POST /users/{uid}/playlists/{kind}/delete`
    pub async fn delete_playlist(&self, kind: impl std::fmt::Display) -> Result<Mutation> {
        let uid = self.resolve_uid(None)?;
        let kind = segment(kind);
        let value = self
            .post_form_value(
                &format!("/users/{uid}/playlists/{kind}/delete"),
                Params::new(),
            )
            .await?;
        Client::acknowledged(&value)
    }

    /// rename a playlist.
    ///
    /// `POST /users/{uid}/playlists/{kind}/name`
    pub async fn rename_playlist(
        &self,
        kind: impl std::fmt::Display,
        name: &str,
    ) -> Result<Playlist> {
        let uid = self.resolve_uid(None)?;
        let kind = segment(kind);
        let form = Params::new().add("value", name);
        self.post_form(&format!("/users/{uid}/playlists/{kind}/name"), form)
            .await
    }

    /// change a playlist's visibility.
    ///
    /// `POST /users/{uid}/playlists/{kind}/visibility`
    pub async fn set_playlist_visibility(
        &self,
        kind: impl std::fmt::Display,
        visibility: Visibility,
    ) -> Result<Playlist> {
        let uid = self.resolve_uid(None)?;
        let kind = segment(kind);
        let form = Params::new().add("value", visibility);
        self.post_form(&format!("/users/{uid}/playlists/{kind}/visibility"), form)
            .await
    }

    /// change a playlist's description.
    ///
    /// `POST /users/{uid}/playlists/{kind}/description`
    pub async fn set_playlist_description(
        &self,
        kind: impl std::fmt::Display,
        description: &str,
    ) -> Result<Playlist> {
        let uid = self.resolve_uid(None)?;
        let kind = segment(kind);
        let form = Params::new().add("value", description);
        self.post_form(&format!("/users/{uid}/playlists/{kind}/description"), form)
            .await
    }

    /// apply a diff to a playlist.
    ///
    /// `POST /users/{uid}/playlists/{kind}/change`
    ///
    /// `revision` must match the playlist's current revision or the api rejects
    /// the change — read it from [`Playlist::revision`] first.
    pub async fn change_playlist(
        &self,
        kind: impl std::fmt::Display,
        diff: &Difference,
        revision: i64,
    ) -> Result<Playlist> {
        let uid = self.resolve_uid(None)?;
        let form = Params::new()
            .add("kind", &kind)
            .add("revision", revision)
            .add("diff", diff.to_json());
        // the form carries the kind verbatim; only the path copy is escaped
        let kind = segment(kind);
        self.post_form(&format!("/users/{uid}/playlists/{kind}/change"), form)
            .await
    }

    /// insert a track into a playlist at the given index.
    ///
    /// a thin wrapper over [`change_playlist`](Self::change_playlist).
    pub async fn add_track_to_playlist(
        &self,
        kind: impl std::fmt::Display,
        track_id: i64,
        album_id: i64,
        at: usize,
        revision: i64,
    ) -> Result<Playlist> {
        let diff = Difference::new().insert(at, [DiffTrack::new(track_id, album_id)]);
        self.change_playlist(kind, &diff, revision).await
    }

    /// remove the tracks in the half-open range `[from, to)` from a playlist.
    pub async fn remove_tracks_from_playlist(
        &self,
        kind: impl std::fmt::Display,
        from: usize,
        to: usize,
        revision: i64,
    ) -> Result<Playlist> {
        let diff = Difference::new().delete(from, to);
        self.change_playlist(kind, &diff, revision).await
    }

    /// join a collaborative playlist as a co-author.
    ///
    /// `POST /playlists/collective/join` — the parameters go in the query even
    /// though this is a `POST`.
    pub async fn join_collective_playlist(&self, owner_uid: i64, token: &str) -> Result<Mutation> {
        let query = Params::new().add("uid", owner_uid).add("token", token);
        let value = self
            .post_form_value_with("/playlists/collective/join", &query, Params::new(), &[])
            .await?;
        Client::acknowledged(&value)
    }

    /// fetch a playlist by its stable uuid.
    ///
    /// `GET /playlist/{uuid}`
    pub async fn playlist_by_uuid(&self, playlist_uuid: &str) -> Result<Playlist> {
        let playlist_uuid = segment(playlist_uuid);
        self.get(&format!("/playlist/{playlist_uuid}"), Params::new())
            .await
    }

    /// fetch entities similar to a playlist.
    ///
    /// `GET /playlist/{uuid}/similar-entities`
    pub async fn playlist_similar_entities(
        &self,
        playlist_uuid: &str,
    ) -> Result<PlaylistSimilarEntities> {
        let playlist_uuid = segment(playlist_uuid);
        self.get(
            &format!("/playlist/{playlist_uuid}/similar-entities"),
            Params::new(),
        )
        .await
    }

    /// fetch playlists by their `uid:kind` ids.
    ///
    /// `GET /playlists`
    pub async fn playlists(
        &self,
        playlist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<PlaylistsList> {
        let joined = playlist_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        self.get("/playlists", Params::new().add("playlistIds", joined))
            .await
    }

    /// fetch a short-form list of playlists by their `uid:kind` ids.
    ///
    /// `POST /playlists/list`
    ///
    /// the returned playlists carry no track lists; use
    /// [`user_playlist`](Self::user_playlist) for one with `tracks` populated.
    pub async fn playlists_list(
        &self,
        playlist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Vec<Playlist>> {
        self.get_batch(BatchObject::Playlist, playlist_ids, Params::new())
            .await
    }

    /// fetch a personal, auto-generated playlist.
    ///
    /// `GET /playlists/personal/{id}`
    ///
    /// known ids: `daily`, `missedLikes`, `recentTracks`, `neverHeard`,
    /// `podcasts`, `origin`.
    pub async fn personal_playlist(&self, playlist_id: &str) -> Result<GeneratedPlaylist> {
        let playlist_id = segment(playlist_id);
        self.get(&format!("/playlists/personal/{playlist_id}"), Params::new())
            .await
    }
}
