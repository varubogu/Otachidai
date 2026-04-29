use crate::db::DbPools;
use crate::i18n::I18n;
use crate::rental::RentalStateMap;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::id::{Id, marker::ApplicationMarker};

pub struct AppState {
    pub db: Arc<DbPools>,
    pub http: Arc<HttpClient>,
    pub application_id: Id<ApplicationMarker>,
    pub i18n: Arc<I18n>,
    pub rental_states: RentalStateMap,
}
