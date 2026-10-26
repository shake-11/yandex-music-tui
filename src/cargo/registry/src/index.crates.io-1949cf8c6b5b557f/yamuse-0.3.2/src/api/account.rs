//! account endpoints.

use std::collections::BTreeMap;

use crate::client::Client;
use crate::error::Result;
use crate::models::{
    Experiments, ExperimentsDetails, PermissionAlerts, PromoCodeStatus, Settings, Status,
    UserSettings,
};
use crate::params::Params;

impl Client {
    /// fetch the status of the authenticated account.
    ///
    /// `GET /account/status`
    pub async fn account_status(&self) -> Result<Status> {
        self.get("/account/status", Params::new()).await
    }

    /// fetch the current user's settings.
    ///
    /// `GET /account/settings`
    pub async fn account_settings(&self) -> Result<UserSettings> {
        self.get("/account/settings", Params::new()).await
    }

    /// change one of the current user's settings.
    ///
    /// `POST /account/settings`
    pub async fn set_account_setting(
        &self,
        name: &str,
        value: impl std::fmt::Display,
    ) -> Result<UserSettings> {
        self.post_form("/account/settings", Params::new().add(name, value))
            .await
    }

    /// change several of the current user's settings at once.
    ///
    /// `POST /account/settings`
    pub async fn set_account_settings(
        &self,
        settings: BTreeMap<String, String>,
    ) -> Result<UserSettings> {
        let form = settings
            .into_iter()
            .fold(Params::new(), |params, (key, value)| {
                params.add(&key, value)
            });
        self.post_form("/account/settings", form).await
    }

    /// fetch purchase offers and payment configuration.
    ///
    /// `GET /settings`
    pub async fn settings(&self) -> Result<Settings> {
        self.get("/settings", Params::new()).await
    }

    /// fetch permission alert messages.
    ///
    /// `GET /permission-alerts`
    pub async fn permission_alerts(&self) -> Result<PermissionAlerts> {
        self.get("/permission-alerts", Params::new()).await
    }

    /// fetch the account's experiment buckets.
    ///
    /// `GET /account/experiments`
    pub async fn account_experiments(&self) -> Result<Experiments> {
        self.get("/account/experiments", Params::new()).await
    }

    /// fetch detailed experiment configuration for the account.
    ///
    /// `GET /account/experiments/details`
    pub async fn account_experiments_details(&self) -> Result<ExperimentsDetails> {
        self.get("/account/experiments/details", Params::new())
            .await
    }

    /// activate a promo code.
    ///
    /// `POST /account/consume-promo-code`
    pub async fn consume_promo_code(&self, code: &str) -> Result<PromoCodeStatus> {
        let form = Params::new()
            .add("code", code)
            .add("language", self.language());
        self.post_form("/account/consume-promo-code", form).await
    }
}
