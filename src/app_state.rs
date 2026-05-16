use crate::db::DbPools;
use crate::i18n::I18n;
use crate::rental::RentalStateMap;
use crate::voice_occupancy::VoiceOccupancyMap;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::id::{Id, marker::ApplicationMarker};

pub struct AppState {
    pub db: Arc<DbPools>,
    pub http: Arc<HttpClient>,
    pub application_id: Id<ApplicationMarker>,
    pub i18n: Arc<I18n>,
    pub config_language: Option<String>,
    pub rental_states: RentalStateMap,
    pub voice_occupancy: VoiceOccupancyMap,
}
