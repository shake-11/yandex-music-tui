//! like and dislike endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Artist, Clip, ClipsWillLike, Like, Mutation, TracksList};
use crate::params::{Params, segment};
use crate::types::{Dislikeable, Likeable};

impl Client {
    /// add or remove a like on one or many objects.
    ///
    /// `POST /users/{uid}/likes/{type}s/{add-multiple|remove}`
    ///
    /// the specialised wrappers below read better at call sites; this is the
    /// general form.
    pub async fn set_liked(
        &self,
        object: Likeable,
        ids: impl IntoIterator<Item = impl std::fmt::Display>,
        liked: bool,
        user_id: Option<i64>,
    ) -> Result<Mutation> {
        let uid = self.resolve_uid(user_id)?;
        let kind = object.as_str();
        let action = if liked { "add-multiple" } else { "remove" };
        self.mutate_batch(
            &format!("/users/{uid}/likes/{kind}s/{action}"),
            &format!("{kind}-ids"),
            ids,
        )
        .await
    }

    /// add or remove a dislike on one or many objects.
    ///
    /// `POST /users/{uid}/dislikes/{type}s/{add-multiple|remove}`
    pub async fn set_disliked(
        &self,
        object: Dislikeable,
        ids: impl IntoIterator<Item = impl std::fmt::Display>,
        disliked: bool,
        user_id: Option<i64>,
    ) -> Result<Mutation> {
        let uid = self.resolve_uid(user_id)?;
        let kind = object.as_str();
        let action = if disliked { "add-multiple" } else { "remove" };
        self.mutate_batch(
            &format!("/users/{uid}/dislikes/{kind}s/{action}"),
            &format!("{kind}-ids"),
            ids,
        )
        .await
    }

    /// like one or many tracks.
    pub async fn like_tracks(
        &self,
        track_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Track, track_ids, true, None).await
    }

    /// remove the like from one or many tracks.
    pub async fn unlike_tracks(
        &self,
        track_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Track, track_ids, false, None)
            .await
    }

    /// like one or many artists.
    pub async fn like_artists(
        &self,
        artist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Artist, artist_ids, true, None)
            .await
    }

    /// remove the like from one or many artists.
    pub async fn unlike_artists(
        &self,
        artist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Artist, artist_ids, false, None)
            .await
    }

    /// like one or many albums.
    pub async fn like_albums(
        &self,
        album_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Album, album_ids, true, None).await
    }

    /// remove the like from one or many albums.
    pub async fn unlike_albums(
        &self,
        album_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Album, album_ids, false, None)
            .await
    }

    /// like one or many playlists, identified as `ownerId:kind`.
    pub async fn like_playlists(
        &self,
        playlist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Playlist, playlist_ids, true, None)
            .await
    }

    /// remove the like from one or many playlists, identified as `ownerId:kind`.
    pub async fn unlike_playlists(
        &self,
        playlist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_liked(Likeable::Playlist, playlist_ids, false, None)
            .await
    }

    /// dislike one or many tracks.
    pub async fn dislike_tracks(
        &self,
        track_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_disliked(Dislikeable::Track, track_ids, true, None)
            .await
    }

    /// remove the dislike from one or many tracks.
    pub async fn undislike_tracks(
        &self,
        track_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_disliked(Dislikeable::Track, track_ids, false, None)
            .await
    }

    /// dislike one or many artists.
    pub async fn dislike_artists(
        &self,
        artist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_disliked(Dislikeable::Artist, artist_ids, true, None)
            .await
    }

    /// remove the dislike from one or many artists.
    pub async fn undislike_artists(
        &self,
        artist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        self.set_disliked(Dislikeable::Artist, artist_ids, false, None)
            .await
    }

    /// fetch the user's liked tracks.
    ///
    /// `GET /users/{uid}/likes/tracks`
    ///
    /// the payload nests the list under `library`; that is unwrapped here.
    pub async fn liked_tracks(&self, user_id: Option<i64>) -> Result<TracksList> {
        let uid = self.resolve_uid(user_id)?;
        let value = self
            .get_value(&format!("/users/{uid}/likes/tracks"), Params::new())
            .await?;
        let library = value.get("library").cloned().unwrap_or(value);
        self.decode(library)
    }

    /// fetch the user's liked albums.
    ///
    /// `GET /users/{uid}/likes/albums`
    pub async fn liked_albums(&self, user_id: Option<i64>) -> Result<Vec<Like>> {
        let uid = self.resolve_uid(user_id)?;
        self.get(&format!("/users/{uid}/likes/albums"), Params::new())
            .await
    }

    /// fetch the user's liked artists.
    ///
    /// `GET /users/{uid}/likes/artists`
    ///
    /// this endpoint returns full artist objects rather than like wrappers.
    pub async fn liked_artists(&self, user_id: Option<i64>) -> Result<Vec<Artist>> {
        let uid = self.resolve_uid(user_id)?;
        self.get(&format!("/users/{uid}/likes/artists"), Params::new())
            .await
    }

    /// fetch the user's liked playlists.
    ///
    /// `GET /users/{uid}/likes/playlists`
    pub async fn liked_playlists(&self, user_id: Option<i64>) -> Result<Vec<Like>> {
        let uid = self.resolve_uid(user_id)?;
        self.get(&format!("/users/{uid}/likes/playlists"), Params::new())
            .await
    }

    /// fetch the user's disliked tracks.
    ///
    /// `GET /users/{uid}/dislikes/tracks`
    pub async fn disliked_tracks(&self, user_id: Option<i64>) -> Result<TracksList> {
        let uid = self.resolve_uid(user_id)?;
        let value = self
            .get_value(&format!("/users/{uid}/dislikes/tracks"), Params::new())
            .await?;
        let library = value.get("library").cloned().unwrap_or(value);
        self.decode(library)
    }

    /// fetch the user's disliked artists.
    ///
    /// `GET /users/{uid}/dislikes/artists`
    pub async fn disliked_artists(&self, user_id: Option<i64>) -> Result<Vec<Artist>> {
        let uid = self.resolve_uid(user_id)?;
        self.get(&format!("/users/{uid}/dislikes/artists"), Params::new())
            .await
    }

    /// fetch a page of the user's liked clips.
    ///
    /// `GET /users/{uid}/likes/clips`
    pub async fn liked_clips(
        &self,
        user_id: Option<i64>,
        page: u32,
        page_size: u32,
    ) -> Result<ClipsWillLike> {
        let uid = self.resolve_uid(user_id)?;
        let params = Params::new().add("page", page).add("pageSize", page_size);
        self.get(&format!("/users/{uid}/likes/clips"), params).await
    }

    /// fetch every liked clip, walking the pages.
    ///
    /// see [`artist_tracks_all`](Self::artist_tracks_all) for the caveats.
    pub async fn liked_clips_all(&self, user_id: Option<i64>, page_size: u32) -> Result<Vec<Clip>> {
        let uid = self.resolve_uid(user_id)?;
        let mut all = Vec::new();
        for page in 0..crate::api::MAX_PAGES {
            let batch = self.liked_clips(Some(uid), page, page_size).await?;
            all.extend(batch.clips);
            if !batch.pager.is_some_and(|pager| pager.has_next_after(page)) {
                break;
            }
        }
        Ok(all)
    }

    /// like a clip.
    ///
    /// `POST /users/{uid}/likes/clips/add`
    pub async fn like_clip(&self, clip_id: impl std::fmt::Display) -> Result<Mutation> {
        let uid = self.resolve_uid(None)?;
        let form = Params::new().add("clip-id", clip_id);
        let value = self
            .post_form_value(&format!("/users/{uid}/likes/clips/add"), form)
            .await?;
        Client::acknowledged_loosely(&value)
    }

    /// remove the like from a clip.
    ///
    /// `POST /users/{uid}/likes/clips/{id}/remove`
    pub async fn unlike_clip(&self, clip_id: impl std::fmt::Display) -> Result<Mutation> {
        let clip_id = segment(clip_id);
        let uid = self.resolve_uid(None)?;
        let value = self
            .post_form_value(
                &format!("/users/{uid}/likes/clips/{clip_id}/remove"),
                Params::new(),
            )
            .await?;
        Client::acknowledged_loosely(&value)
    }
}
