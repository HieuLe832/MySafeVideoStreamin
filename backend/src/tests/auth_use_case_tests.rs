use crate::application::use_cases::auth::VerifyTokenUseCase;
use crate::domain::ports::auth_verifier::AuthVerifier;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

struct MockAuthVerifier {
    verify_result: Mutex<Result<String, String>>,
    call_count: Arc<AtomicUsize>,
}

#[axum::async_trait]
impl AuthVerifier for MockAuthVerifier {
    async fn verify_token(&self, _token: &str) -> Result<String, String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.verify_result.lock().unwrap().clone()
    }
}

#[tokio::test]
async fn test_verify_token_success() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let mock_verifier = Arc::new(MockAuthVerifier {
        verify_result: Mutex::new(Ok("admin@example.com".to_string())),
        call_count: call_count.clone(),
    });

    let use_case = VerifyTokenUseCase::new(mock_verifier, "admin@example.com".to_string());
    
    let result = use_case.execute("valid-token").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "admin@example.com");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_verify_token_unauthorized_email() {
    let call_count = Arc::new(AtomicUsize::new(0));
    // Verify succeeds but returns a different email address (unauthorized)
    let mock_verifier = Arc::new(MockAuthVerifier {
        verify_result: Mutex::new(Ok("stranger@gmail.com".to_string())),
        call_count: call_count.clone(),
    });

    let use_case = VerifyTokenUseCase::new(mock_verifier, "admin@example.com".to_string());
    
    let result = use_case.execute("valid-token").await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("không được phép truy cập"));
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_verify_token_verification_failed() {
    let call_count = Arc::new(AtomicUsize::new(0));
    // Verify fails at the google API level
    let mock_verifier = Arc::new(MockAuthVerifier {
        verify_result: Mutex::new(Err("Token expired".to_string())),
        call_count: call_count.clone(),
    });

    let use_case = VerifyTokenUseCase::new(mock_verifier, "admin@example.com".to_string());
    
    let result = use_case.execute("expired-token").await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Token expired");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_verify_token_cache_hit() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let mock_verifier = Arc::new(MockAuthVerifier {
        verify_result: Mutex::new(Ok("admin@example.com".to_string())),
        call_count: call_count.clone(),
    });

    let use_case = VerifyTokenUseCase::new(mock_verifier, "admin@example.com".to_string());
    
    // First call: cache miss, calls verify_token
    let result1 = use_case.execute("cached-token").await;
    assert!(result1.is_ok());
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Second call: cache hit, should NOT call verify_token again
    let result2 = use_case.execute("cached-token").await;
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), "admin@example.com");
    assert_eq!(call_count.load(Ordering::SeqCst), 1); // call count is still 1!
}

#[tokio::test]
async fn test_verify_token_negative_cache_hit() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let mock_verifier = Arc::new(MockAuthVerifier {
        verify_result: Mutex::new(Err("Invalid credentials".to_string())),
        call_count: call_count.clone(),
    });

    let use_case = VerifyTokenUseCase::new(mock_verifier, "admin@example.com".to_string());

    // First call: cache miss, calls verify_token, returns Err
    let result1 = use_case.execute("bad-token").await;
    assert!(result1.is_err());
    assert_eq!(result1.unwrap_err(), "Invalid credentials");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    // Second call: cache hit (negative caching), should immediately return Err from cache without calling verify_token
    let result2 = use_case.execute("bad-token").await;
    assert!(result2.is_err());
    assert_eq!(result2.unwrap_err(), "Invalid credentials");
    assert_eq!(call_count.load(Ordering::SeqCst), 1); // Call count remains 1!
}
