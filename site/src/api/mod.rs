pub mod routes;
pub mod middleware;
pub mod types;
pub mod error;
pub mod content_negotiation;
pub mod schemas;
pub mod validation;
pub mod openapi;
pub mod utilities;

pub use routes::{create_api_router, create_cupboard_api_router};
pub use middleware::{content_negotiation_middleware, logging_middleware, metrics_middleware};
pub use types::{ApiResponse, ApiError, PaginationParams, ApiResult};
pub use error::{ApiErrorType, handle_service_error};
pub use content_negotiation::{ContentType, negotiate_content_type};
pub use schemas::*;
pub use validation::{ValidatedJson, ValidationHelper};
pub use openapi::{ApiDoc, generate_openapi_spec};
pub use utilities::{
    PaginationHelper, QueryHelper, ResponseHelper, UrlHelper, 
    SearchParams, ApiResponseHelper, PaginatedListResponse,
    RateLimitHelper, ContentTypeHelper
};