pub mod content_negotiation;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod schemas;
pub mod types;
pub mod utilities;
pub mod validation;

#[cfg(test)]
mod tests;

pub use content_negotiation::{negotiate_content_type, ContentType};
pub use error::{handle_service_error, ApiErrorType};
pub use middleware::{content_negotiation_middleware, logging_middleware, metrics_middleware};
pub use openapi::{generate_openapi_spec, ApiDoc};
pub use routes::{create_api_router, create_cupboard_api_router};
pub use schemas::*;
pub use types::{ApiError, ApiResponse, ApiResult, PaginationParams};
pub use utilities::{
    ApiResponseHelper, ContentTypeHelper, PaginatedListResponse, PaginationHelper, QueryHelper,
    RateLimitHelper, ResponseHelper, SearchParams, UrlHelper,
};
pub use validation::{ValidatedJson, ValidationHelper};
