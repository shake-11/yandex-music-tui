//! account status, subscription state and user settings.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::user::User;

/// the set of permissions granted to the account.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Permissions {
    /// date until which the permissions are valid.
    pub until: Option<String>,
    /// currently granted permission values.
    pub values: Vec<String>,
    /// default permission values.
    pub default: Vec<String>,
}

/// yandex plus status of the account.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Plus {
    /// whether the account has plus.
    pub has_plus: Option<bool>,
    /// whether the onboarding tutorial is completed.
    pub is_tutorial_completed: Option<bool>,
    /// whether the account was migrated to the current plus subscription.
    pub migrated: Option<bool>,
}

/// a monetary amount in a given currency.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Price {
    /// amount, in the currency's minor units as the api reports it.
    ///
    /// a float because the api serializes it as a json number and does send
    /// fractional values.
    pub amount: Option<f64>,
    /// currency code.
    pub currency: Option<String>,
}

/// one part of a product's licence text, optionally a link.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LicenceTextPart {
    /// text fragment.
    pub text: Option<String>,
    /// link target, when the fragment is a link.
    pub url: Option<String>,
}

/// a purchasable subscription product.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Product {
    /// product id.
    pub product_id: Option<String>,
    /// product type.
    pub r#type: Option<String>,
    /// subscription duration.
    pub duration: Option<i64>,
    /// trial duration.
    pub trial_duration: Option<i64>,
    /// feature key.
    pub feature: Option<String>,
    /// whether this is a debug product.
    pub debug: Option<bool>,
    /// whether the product includes plus.
    pub plus: Option<bool>,
    /// recurring price.
    pub price: Option<Price>,
    /// common period duration.
    pub common_period_duration: Option<String>,
    /// whether this is the cheapest product.
    pub cheapest: Option<bool>,
    /// display title.
    pub title: Option<String>,
    /// whether this is a family subscription.
    pub family_sub: Option<bool>,
    /// facebook share image.
    pub fb_image: Option<String>,
    /// facebook share name.
    pub fb_name: Option<String>,
    /// whether the product is a family plan.
    pub family: Option<bool>,
    /// feature keys.
    pub features: Vec<String>,
    /// description.
    pub description: Option<String>,
    /// whether the product is available.
    pub available: Option<bool>,
    /// whether a trial is available.
    pub trial_available: Option<bool>,
    /// trial period duration.
    pub trial_period_duration: Option<String>,
    /// intro period duration.
    pub intro_period_duration: Option<String>,
    /// intro price.
    pub intro_price: Option<Price>,
    /// start period duration.
    pub start_period_duration: Option<String>,
    /// start price.
    pub start_price: Option<Price>,
    /// licence text parts.
    pub licence_text_parts: Vec<LicenceTextPart>,
    /// whether a vendor trial is available.
    pub vendor_trial_available: Option<bool>,
    /// call-to-action button text.
    pub button_text: Option<String>,
    /// additional button text.
    pub button_additional_text: Option<String>,
    /// accepted payment-method types.
    pub payment_method_types: Vec<String>,
}

/// an auto-renewing subscription.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoRenewable {
    /// expiry timestamp.
    pub expires: Option<String>,
    /// vendor name.
    pub vendor: Option<String>,
    /// vendor help url.
    pub vendor_help_url: Option<String>,
    /// the subscribed product.
    pub product: Option<Product>,
    /// whether the subscription has finished.
    pub finished: Option<bool>,
    /// owner of a shared (family) subscription.
    pub master_info: Option<User>,
    /// product id.
    pub product_id: Option<String>,
    /// order id.
    pub order_id: Option<i64>,
}

/// a non-auto-renewing subscription window.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NonAutoRenewable {
    /// start timestamp.
    pub start: Option<String>,
    /// end timestamp.
    pub end: Option<String>,
}

/// the remaining days of a non-auto-renewing subscription.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RenewableRemainder {
    /// days remaining.
    pub days: Option<i64>,
}

/// instructions for deactivating an operator subscription.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Deactivation {
    /// deactivation method; `ussd` is the known value.
    pub method: Option<String>,
    /// human-readable instructions.
    pub instructions: Option<String>,
}

/// a mobile-operator-billed subscription.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Operator {
    /// product id.
    pub product_id: Option<String>,
    /// billed phone number.
    pub phone: Option<String>,
    /// payment regularity.
    pub payment_regularity: Option<String>,
    /// deactivation options.
    pub deactivation: Vec<Deactivation>,
    /// display title.
    pub title: Option<String>,
    /// whether the subscription is suspended.
    pub suspended: Option<bool>,
}

/// the account's subscription state.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Subscription {
    /// remaining days of the non-auto-renewing subscription.
    pub non_auto_renewable_remainder: Option<RenewableRemainder>,
    /// auto-renewing subscriptions.
    pub auto_renewable: Vec<AutoRenewable>,
    /// family auto-renewing subscriptions.
    pub family_auto_renewable: Vec<AutoRenewable>,
    /// whether the user ever had any subscription.
    pub had_any_subscription: Option<bool>,
    /// operator-billed subscriptions.
    pub operator: Vec<Operator>,
    /// the non-auto-renewing subscription, if any.
    pub non_auto_renewable: Option<NonAutoRenewable>,
    /// whether a trial can be started.
    pub can_start_trial: Option<bool>,
    /// mcdonald's promo flag.
    pub mcdonalds: Option<bool>,
    /// subscription end timestamp.
    pub end: Option<String>,
}

/// a phone number attached to a yandex passport account.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PassportPhone {
    /// the phone number.
    pub phone: Option<String>,
}

/// information about the authenticated account.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Account {
    /// server timestamp of the response.
    pub now: Option<String>,
    /// whether the music service is available for the account.
    pub service_available: Option<bool>,
    /// region code.
    pub region: Option<i64>,
    /// account uid.
    pub uid: Option<i64>,
    /// login.
    pub login: Option<String>,
    /// full name.
    pub full_name: Option<String>,
    /// second name.
    pub second_name: Option<String>,
    /// first name.
    pub first_name: Option<String>,
    /// display name.
    pub display_name: Option<String>,
    /// whether this is a hosted (organization) user.
    pub hosted_user: Option<bool>,
    /// birthday, as an iso date.
    pub birthday: Option<String>,
    /// phone numbers on the passport.
    pub passport_phones: Vec<PassportPhone>,
    /// registration timestamp.
    pub registered_at: Option<String>,
    /// whether appmetrica info is available.
    pub has_info_for_app_metrica: Option<bool>,
    /// whether this is a child account.
    pub child: Option<bool>,
    /// region code.
    pub region_code: Option<String>,
    /// whether the account is a non-owner family-plan member.
    pub non_owner_family_member: Option<bool>,
}

/// aggregated account status returned by `account/status`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Status {
    /// account info.
    pub account: Option<Account>,
    /// granted permissions.
    pub permissions: Option<Permissions>,
    /// advertisement payload.
    pub advertisement: Option<String>,
    /// subscription state.
    pub subscription: Option<Subscription>,
    /// offline cache track limit.
    pub cache_limit: Option<i64>,
    /// whether the user is a sub-editor.
    pub subeditor: Option<bool>,
    /// sub-editor level.
    pub subeditor_level: Option<i64>,
    /// yandex plus status.
    pub plus: Option<Plus>,
    /// default email.
    pub default_email: Option<String>,
    /// allowed skips per hour.
    pub skips_per_hour: Option<i64>,
    /// whether a personal station exists.
    pub station_exists: Option<bool>,
    /// station data, still free-form upstream.
    pub station_data: Option<Value>,
    /// bar-below alert, still free-form upstream.
    pub bar_below: Option<Value>,
    /// premium region.
    pub premium_region: Option<i64>,
    /// experiment bucket.
    pub experiment: Option<i64>,
    /// whether a pre-trial is active.
    pub pretrial_active: Option<bool>,
    /// user hash.
    pub userhash: Option<String>,
    /// masterhub payload, still free-form upstream.
    pub masterhub: Option<Value>,
    /// the account's purchase options.
    ///
    /// named as a flag, but the live api sends the options themselves — see
    /// [`FlagOrList`](super::FlagOrList).
    pub has_options: Option<super::FlagOrList>,
}

impl Status {
    /// the account uid, when the status carries an account block.
    #[must_use]
    pub fn uid(&self) -> Option<i64> {
        self.account.as_ref()?.uid
    }

    /// whether the account currently has yandex plus.
    #[must_use]
    pub fn has_plus(&self) -> bool {
        self.plus.as_ref().and_then(|p| p.has_plus).unwrap_or(false)
    }
}

/// a user's playback and library preferences.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UserSettings {
    /// owner uid.
    pub uid: Option<i64>,
    /// whether last.fm scrobbling is enabled.
    pub last_fm_scrobbling_enabled: Option<bool>,
    /// whether shuffle is enabled.
    pub shuffle_enabled: Option<bool>,
    /// default playback volume, in percent.
    pub volume_percents: Option<i64>,
    /// last modification timestamp.
    pub modified: Option<String>,
    /// whether facebook scrobbling is enabled.
    pub facebook_scrobbling_enabled: Option<bool>,
    /// whether newly added tracks go to the top of playlists.
    pub add_new_track_on_playlist_top: Option<bool>,
    /// visibility of the user's music.
    pub user_music_visibility: Option<String>,
    /// visibility of the user's social activity.
    pub user_social_visibility: Option<String>,
    /// whether ringback tones are disabled.
    pub rbt_disabled: Option<bool>,
    /// ui theme.
    pub theme: Option<String>,
    /// whether promos are disabled.
    pub promos_disabled: Option<bool>,
    /// whether radio auto-plays.
    pub auto_play_radio: Option<bool>,
    /// whether queue sync is enabled.
    pub sync_queue_enabled: Option<bool>,
    /// whether ads are disabled.
    pub ads_disabled: Option<bool>,
    /// whether yandex disk integration is enabled.
    pub disk_enabled: Option<bool>,
    /// whether disk tracks appear in the library.
    pub show_disk_tracks_in_library: Option<bool>,
    /// whether explicit content is forbidden for the account.
    pub explicit_forbidden: Option<bool>,
    /// whether child mode is enabled.
    pub child_mod_enabled: Option<bool>,
    /// whether the onboarding wizard has been completed.
    pub wizard_is_passed: Option<bool>,
    /// library accent hue chosen by the user.
    pub user_collection_hue: Option<String>,
    /// whether child mode was changed by the user.
    pub child_mode_changed_by_user: Option<bool>,
}

/// purchase offers and payment configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// in-app subscription products, still free-form upstream.
    pub in_app_products: Option<Value>,
    /// native subscription products, still free-form upstream.
    pub native_products: Option<Value>,
    /// web payment url.
    pub web_payment_url: Option<String>,
    /// whether promo codes are enabled.
    pub promo_codes_enabled: Option<bool>,
    /// monthly web payment price, still free-form upstream.
    pub web_payment_month_product_price: Option<Value>,
    /// identifier of the offers batch presented to the account.
    pub offers_batch_id: Option<String>,
}

/// permission alert messages.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PermissionAlerts {
    /// alert messages.
    pub alerts: Vec<String>,
}

/// result of activating a promo code.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PromoCodeStatus {
    /// activation status.
    pub status: Option<String>,
    /// human-readable status description.
    pub status_desc: Option<String>,
    /// updated account status.
    pub account_status: Option<Status>,
}

/// a/b experiment buckets for the account.
///
/// the api returns a free-form map of experiment names to values, so the payload
/// is preserved verbatim.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Experiments {
    /// the raw experiment map.
    pub values: serde_json::Map<String, Value>,
}

/// detailed a/b experiment configuration for the account.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExperimentsDetails {
    /// the raw detailed experiment map.
    pub values: serde_json::Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_exposes_uid_and_plus() {
        let status: Status =
            serde_json::from_str(r#"{"account":{"uid":123,"login":"me"},"plus":{"hasPlus":true}}"#)
                .unwrap();
        assert_eq!(status.uid(), Some(123));
        assert!(status.has_plus());
    }

    #[test]
    fn a_status_without_plus_reports_false_rather_than_unknown() {
        let empty: Status = serde_json::from_str("{}").unwrap();
        assert!(!empty.has_plus());
        assert_eq!(empty.uid(), None);

        let no_plus: Status = serde_json::from_str(r#"{"plus":{"hasPlus":false}}"#).unwrap();
        assert!(!no_plus.has_plus());
    }

    #[test]
    fn free_form_blocks_are_kept_rather_than_dropped() {
        let status: Status =
            serde_json::from_str(r#"{"stationData":{"anything":[1,2]},"barBelow":"text"}"#)
                .unwrap();
        assert!(status.station_data.is_some());
        assert!(status.bar_below.is_some());
    }

    #[test]
    fn experiments_preserve_the_whole_map() {
        let experiments: Experiments =
            serde_json::from_str(r#"{"exp_a":"on","exp_b":"off"}"#).unwrap();
        assert_eq!(experiments.values.len(), 2);
        assert_eq!(experiments.values["exp_a"], Value::String("on".into()));
    }

    #[test]
    fn subscription_blocks_deserialize_in_full() {
        let subscription: Subscription = serde_json::from_str(
            r#"{
                "autoRenewable":[{"expires":"2027-01-01","product":{"productId":"p1","price":{"amount":299,"currency":"RUB"}}}],
                "nonAutoRenewableRemainder":{"days":12},
                "hadAnySubscription":true
            }"#,
        )
        .unwrap();
        assert_eq!(subscription.auto_renewable.len(), 1);
        let product = subscription.auto_renewable[0].product.as_ref().unwrap();
        assert_eq!(product.price.as_ref().unwrap().amount, Some(299.0));
        assert_eq!(
            subscription.non_auto_renewable_remainder.unwrap().days,
            Some(12)
        );
    }

    #[test]
    fn every_account_model_survives_an_empty_object() {
        serde_json::from_str::<Status>("{}").unwrap();
        serde_json::from_str::<Account>("{}").unwrap();
        serde_json::from_str::<UserSettings>("{}").unwrap();
        serde_json::from_str::<Settings>("{}").unwrap();
        serde_json::from_str::<Subscription>("{}").unwrap();
        serde_json::from_str::<Product>("{}").unwrap();
        serde_json::from_str::<PromoCodeStatus>("{}").unwrap();
        serde_json::from_str::<PermissionAlerts>("{}").unwrap();
    }
}
