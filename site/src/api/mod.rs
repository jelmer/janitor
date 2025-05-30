pub mod routes;
pub mod middleware;
pub mod types;
pub mod error;
pub mod content_negotiation;

pub use routes::{create_api_router, create_cupboard_api_router};
pub use middleware::{content_negotiation_middleware, logging_middleware, metrics_middleware};
pub use types::{ApiResponse, ApiError, PaginationParams, ApiResult};
pub use error::{ApiErrorType, handle_service_error};
pub use content_negotiation::{ContentType, negotiate_content_type};