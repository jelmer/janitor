//! Authentication extractors for Axum request handlers

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};

use super::{AuthContext, AuthError, UserContext, WorkerAuth};

/// Extractor for worker authentication
///
/// This extractor can be used in Axum handlers to require worker authentication:
/// ```rust,ignore
/// async fn handler(Worker(worker): Worker) -> Result<String, AuthError> {
///     Ok(format!("Hello, worker: {}", worker.name))
/// }
/// ```
pub struct Worker(pub WorkerAuth);

impl<S> FromRequestParts<S> for Worker
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Check if worker auth is already in extensions (set by middleware)
        if let Some(worker) = parts.extensions.get::<WorkerAuth>() {
            Ok(Worker(worker.clone()))
        } else {
            Err(AuthError::MissingAuth.into_response())
        }
    }
}

/// Extractor for optional worker authentication
pub struct OptionalWorker(pub Option<WorkerAuth>);

impl<S> FromRequestParts<S> for OptionalWorker
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Always succeeds, returns None if no worker auth
        let worker = parts.extensions.get::<WorkerAuth>().cloned();
        Ok(OptionalWorker(worker))
    }
}

/// Extractor for user authentication
///
/// This extractor can be used in Axum handlers to require user authentication:
/// ```rust,ignore
/// async fn handler(User(user): User) -> Result<String, AuthError> {
///     Ok(format!("Hello, {}", user.email.unwrap_or_default()))
/// }
/// ```
pub struct User(pub UserContext);

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Check if user context is in extensions (set by middleware)
        if let Some(user) = parts.extensions.get::<UserContext>() {
            Ok(User(user.clone()))
        } else if let Some(auth_context) = parts.extensions.get::<AuthContext>() {
            // Extract user from auth context
            match auth_context {
                AuthContext::User(user) => Ok(User(user.clone())),
                _ => Err(AuthError::InvalidCredentials.into_response()),
            }
        } else {
            Err(AuthError::MissingAuth.into_response())
        }
    }
}

/// Extractor for optional user authentication
pub struct OptionalUser(pub Option<UserContext>);

impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Always succeeds, returns None if no user auth
        let user = if let Some(user) = parts.extensions.get::<UserContext>() {
            Some(user.clone())
        } else if let Some(auth_context) = parts.extensions.get::<AuthContext>() {
            match auth_context {
                AuthContext::User(user) => Some(user.clone()),
                _ => None,
            }
        } else {
            None
        };

        Ok(OptionalUser(user))
    }
}

/// Extractor for any authentication context
///
/// This extractor accepts any type of authentication (worker, user, API key):
/// ```rust,ignore
/// async fn handler(Auth(context): Auth) -> Result<String, AuthError> {
///     Ok(format!("Authenticated as: {}", context.identity()))
/// }
/// ```
pub struct Auth(pub AuthContext);

impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Check if auth context is in extensions (set by middleware)
        if let Some(context) = parts.extensions.get::<AuthContext>() {
            if matches!(context, AuthContext::Anonymous) {
                Err(AuthError::MissingAuth.into_response())
            } else {
                Ok(Auth(context.clone()))
            }
        } else {
            Err(AuthError::MissingAuth.into_response())
        }
    }
}

/// Extractor for optional authentication context
pub struct OptionalAuth(pub Option<AuthContext>);

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Always succeeds
        let context = parts.extensions.get::<AuthContext>().cloned();
        Ok(OptionalAuth(context))
    }
}

/// Admin user extractor - requires admin role
pub struct AdminUser(pub UserContext);

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // First get user context
        let User(user) = User::from_request_parts(parts, _state).await?;

        // Check if user has admin role
        if user.is_admin() {
            Ok(AdminUser(user))
        } else {
            Err(AuthError::Unauthorized.into_response())
        }
    }
}

/// QA reviewer extractor - requires QA reviewer or admin role
pub struct QaUser(pub UserContext);

impl<S> FromRequestParts<S> for QaUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // First get user context
        let User(user) = User::from_request_parts(parts, _state).await?;

        // Check if user has QA reviewer or admin role
        if user.is_qa_reviewer() || user.is_admin() {
            Ok(QaUser(user))
        } else {
            Err(AuthError::Unauthorized.into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserRole;
    use axum::http::Request;

    #[tokio::test]
    async fn test_worker_extractor() {
        let worker = WorkerAuth {
            name: "test-worker".to_string(),
            link: None,
        };

        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(worker.clone());

        let result = Worker::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.name, "test-worker");
    }

    #[tokio::test]
    async fn test_optional_worker_extractor() {
        let (mut parts, _) = Request::new(()).into_parts();

        let result = OptionalWorker::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().0.is_none());
    }

    #[tokio::test]
    async fn test_user_extractor_from_context() {
        let user = UserContext {
            id: "user123".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            roles: vec![UserRole::User],
            expires_at: None,
        };

        let auth_context = AuthContext::User(user.clone());

        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(auth_context);

        let result = User::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.id, "user123");
    }

    #[tokio::test]
    async fn test_admin_user_extractor() {
        let user = UserContext {
            id: "admin123".to_string(),
            email: Some("admin@example.com".to_string()),
            name: Some("Admin User".to_string()),
            roles: vec![super::super::UserRole::Admin],
            expires_at: None,
        };

        let (mut parts, _) = Request::new(()).into_parts();
        parts.extensions.insert(user);

        let result = AdminUser::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.id, "admin123");
    }
}
